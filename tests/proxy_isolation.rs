//! Loopback endpoints are a local trust boundary, even with system proxies.

use std::{
    io::{Read, Write},
    net::TcpListener,
    process::Command,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread,
    time::Duration,
};

use zai_rs::{
    client::{ApiFamily, HttpTransportConfig, ZaiClient},
    file::{FileListPurpose, FileListRequest},
    usage::CodingPlanUsageRequest,
};

const CHILD_MARKER: &str = "ZAI_RS_PROXY_ISOLATION_CHILD";
const TARGET_URL: &str = "ZAI_RS_PROXY_ISOLATION_TARGET";
const TEST_KEY: &str = "test.12345678901234567890";
const LOOPBACK_ONLY: &str = "loopback-only";
const MIXED_ENDPOINTS: &str = "mixed-endpoints";

#[derive(Debug)]
struct Observation {
    connected: bool,
    authorization_seen: bool,
}

fn spawn_http_endpoint(
    listener: TcpListener,
    status: u16,
    body: &'static str,
    stop: Arc<AtomicBool>,
) -> (thread::JoinHandle<()>, mpsc::Receiver<Observation>) {
    let (observed_tx, observed_rx) = mpsc::channel();
    let join = thread::spawn(move || {
        listener.set_nonblocking(true).unwrap();
        while !stop.load(Ordering::Acquire) {
            match listener.accept() {
                Ok((mut stream, _)) => {
                    stream
                        .set_read_timeout(Some(Duration::from_secs(2)))
                        .unwrap();
                    let mut request = Vec::new();
                    let mut chunk = [0_u8; 4096];
                    loop {
                        match stream.read(&mut chunk) {
                            Ok(0) => break,
                            Ok(read) => {
                                request.extend_from_slice(&chunk[..read]);
                                if request.windows(4).any(|window| window == b"\r\n\r\n") {
                                    break;
                                }
                            },
                            Err(error)
                                if matches!(
                                    error.kind(),
                                    std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                                ) =>
                            {
                                break;
                            },
                            Err(_) => break,
                        }
                    }
                    let authorization_seen = String::from_utf8_lossy(&request)
                        .lines()
                        .any(|line| line.to_ascii_lowercase().starts_with("authorization:"));
                    let reason = if status == 200 { "OK" } else { "Bad Gateway" };
                    let response = format!(
                        "HTTP/1.1 {status} {reason}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                        body.len()
                    );
                    let _ = stream.write_all(response.as_bytes());
                    let _ = stream.flush();
                    let _ = observed_tx.send(Observation {
                        connected: true,
                        authorization_seen,
                    });
                    return;
                },
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(5));
                },
                Err(_) => break,
            }
        }
        let _ = observed_tx.send(Observation {
            connected: false,
            authorization_seen: false,
        });
    });
    (join, observed_rx)
}

fn run_child_request() {
    let target = std::env::var(TARGET_URL).expect("parent did not provide the target URL");
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async move {
        let client = ZaiClient::builder(TEST_KEY)
            .allow_insecure_transport(true)
            .endpoint(ApiFamily::PaasV4, format!("{target}/api/paas/v4"))
            .build()
            .unwrap();
        FileListRequest::new(FileListPurpose::Batch)
            .send_via(&client)
            .await
            .expect("the direct loopback target must answer the request");
    });
}

fn run_mixed_child_requests() {
    let target = std::env::var(TARGET_URL).expect("parent did not provide the target URL");
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async move {
        let client = ZaiClient::builder(TEST_KEY)
            .allow_insecure_transport(true)
            .endpoint(ApiFamily::PaasV4, format!("{target}/api/paas/v4"))
            .endpoint(ApiFamily::Monitor, "https://public.invalid/api/monitor")
            .transport(HttpTransportConfig::default().with_max_attempts(1).unwrap())
            .build()
            .unwrap();

        FileListRequest::new(FileListPurpose::Batch)
            .send_via(&client)
            .await
            .expect("the loopback family must use the direct pool");
        CodingPlanUsageRequest::new()
            .send_via(&client)
            .await
            .expect_err("the scripted system proxy intentionally rejects CONNECT");
    });
}

#[test]
fn loopback_request_bypasses_system_proxy() {
    if std::env::var(CHILD_MARKER).as_deref() == Ok(LOOPBACK_ONLY) {
        run_child_request();
        return;
    }

    let target_listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
    let target_address = target_listener.local_addr().unwrap();
    let proxy_listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
    let proxy_address = proxy_listener.local_addr().unwrap();
    let stop = Arc::new(AtomicBool::new(false));

    let (target_join, target_observed) = spawn_http_endpoint(
        target_listener,
        200,
        r#"{"object":"list","data":[]}"#,
        Arc::clone(&stop),
    );
    // A vulnerable client receives a prompt terminal response instead of
    // waiting for its 60-second request deadline. The parent assertions below
    // still prove that no connection (and no Authorization header) reached it.
    let (proxy_join, proxy_observed) = spawn_http_endpoint(
        proxy_listener,
        502,
        r#"{"error":{"message":"proxy must not receive loopback traffic"}}"#,
        Arc::clone(&stop),
    );

    let proxy_url = format!("http://{proxy_address}");
    let output = Command::new(std::env::current_exe().unwrap())
        .arg("--exact")
        .arg("loopback_request_bypasses_system_proxy")
        .arg("--nocapture")
        .env(CHILD_MARKER, LOOPBACK_ONLY)
        .env(TARGET_URL, format!("http://{target_address}"))
        .env("HTTP_PROXY", &proxy_url)
        .env("http_proxy", &proxy_url)
        .env("ALL_PROXY", &proxy_url)
        .env("all_proxy", &proxy_url)
        .env("NO_PROXY", "")
        .env("no_proxy", "")
        .output()
        .unwrap();

    stop.store(true, Ordering::Release);
    target_join.join().unwrap();
    proxy_join.join().unwrap();
    let target = target_observed.recv().unwrap();
    let proxy = proxy_observed.recv().unwrap();

    assert!(
        output.status.success(),
        "isolated child request failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(target.connected, "the direct target received no request");
    assert!(
        target.authorization_seen,
        "the direct target did not receive SDK authentication"
    );
    assert!(
        !proxy.connected,
        "the configured proxy received loopback traffic"
    );
    assert!(
        !proxy.authorization_seen,
        "the configured proxy received the SDK Authorization header"
    );
}

#[test]
fn mixed_client_keeps_public_https_on_system_proxy() {
    if std::env::var(CHILD_MARKER).as_deref() == Ok(MIXED_ENDPOINTS) {
        run_mixed_child_requests();
        return;
    }

    let target_listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
    let target_address = target_listener.local_addr().unwrap();
    let proxy_listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
    let proxy_address = proxy_listener.local_addr().unwrap();
    let stop = Arc::new(AtomicBool::new(false));

    let (target_join, target_observed) = spawn_http_endpoint(
        target_listener,
        200,
        r#"{"object":"list","data":[]}"#,
        Arc::clone(&stop),
    );
    let (proxy_join, proxy_observed) = spawn_http_endpoint(
        proxy_listener,
        502,
        r#"{"error":{"message":"scripted CONNECT rejection"}}"#,
        Arc::clone(&stop),
    );

    let proxy_url = format!("http://{proxy_address}");
    let output = Command::new(std::env::current_exe().unwrap())
        .arg("--exact")
        .arg("mixed_client_keeps_public_https_on_system_proxy")
        .arg("--nocapture")
        .env(CHILD_MARKER, MIXED_ENDPOINTS)
        .env(TARGET_URL, format!("http://{target_address}"))
        .env("HTTP_PROXY", &proxy_url)
        .env("http_proxy", &proxy_url)
        .env("HTTPS_PROXY", &proxy_url)
        .env("https_proxy", &proxy_url)
        .env("ALL_PROXY", &proxy_url)
        .env("all_proxy", &proxy_url)
        .env("NO_PROXY", "")
        .env("no_proxy", "")
        .output()
        .unwrap();

    stop.store(true, Ordering::Release);
    target_join.join().unwrap();
    proxy_join.join().unwrap();
    let target = target_observed.recv().unwrap();
    let proxy = proxy_observed.recv().unwrap();

    assert!(
        output.status.success(),
        "isolated mixed-endpoint child failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(target.connected, "the loopback target received no request");
    assert!(
        target.authorization_seen,
        "the direct loopback target did not receive SDK authentication"
    );
    assert!(
        proxy.connected,
        "the public HTTPS request did not use the configured system proxy"
    );
    assert!(
        !proxy.authorization_seen,
        "the SDK Authorization header must not be sent on the proxy CONNECT"
    );
}
