//! Linux-only isolated RSS gate for the 100 MiB atomic download path.

#![cfg(target_os = "linux")]

mod support;

use std::io::{Read as _, Write as _};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::time::{Duration, Instant};

use bytes::Bytes;
use serde_json::Value;
use support::http_server::{ScriptedResponse, TestServer};
use zai_rs::client::{ApiFamily, ZaiClient};
use zai_rs::file::FileContentRequest;

const TEST_NAME: &str = "linux_file_download_100_mib_rss_gate";
const CHILD_MARKER: &str = "ZAI_RS_FILE_DOWNLOAD_RSS_CHILD";
const BASE_URL_ENV: &str = "ZAI_RS_FILE_DOWNLOAD_RSS_BASE_URL";
const DESTINATION_ENV: &str = "ZAI_RS_FILE_DOWNLOAD_RSS_DESTINATION";
const METRIC_PREFIX: &str = "ZAI_FILE_DOWNLOAD_RSS_METRIC=";
const KEY: &str = "test.12345678901234567890";
const TOTAL_BYTES: usize = 100 * 1024 * 1024;
const CHUNK_BYTES: usize = 64 * 1024;
const MAX_DELTA_KIB: u64 = 32 * 1024;
const CHILD_LIVENESS_TIMEOUT: Duration = Duration::from_secs(5 * 60);

#[tokio::test]
#[ignore = "Linux RSS gate; run this integration test with --release --ignored"]
async fn linux_file_download_100_mib_rss_gate() {
    if cfg!(debug_assertions) {
        panic!("the RSS gate must run with --release");
    }

    if std::env::var_os(CHILD_MARKER).is_some() {
        run_download_child().await;
    } else {
        run_server_parent().await;
    }
}

async fn run_download_child() {
    let base_url = std::env::var(BASE_URL_ENV).expect("parent did not provide the server URL");
    let destination = PathBuf::from(
        std::env::var_os(DESTINATION_ENV).expect("parent did not provide the destination"),
    );
    let client = ZaiClient::builder(KEY)
        .allow_insecure_transport(true)
        .endpoint(ApiFamily::PaasV4, format!("{base_url}/api/paas/v4"))
        .build()
        .expect("RSS child client configuration must be valid");

    let pid = std::process::id();
    let baseline_vm_hwm_kib = read_vm_hwm_kib(pid);
    let started = Instant::now();
    let written = FileContentRequest::new("rss-gate")
        .send_to_via(&client, &destination)
        .await
        .expect("100 MiB streaming download failed");
    let elapsed_seconds = started.elapsed().as_secs_f64();
    let throughput_mib_s =
        (written as f64 / (1024 * 1024) as f64) / elapsed_seconds.max(f64::MIN_POSITIVE);
    let final_vm_hwm_kib = read_vm_hwm_kib(pid);
    let delta_vm_hwm_kib = final_vm_hwm_kib
        .checked_sub(baseline_vm_hwm_kib)
        .expect("VmHWM must be monotonic within one process");

    let metric = serde_json::json!({
        "schema": "zai-rs.file-download-rss.v1",
        "pid": pid,
        "bytes": written,
        "baseline_vm_hwm_kib": baseline_vm_hwm_kib,
        "final_vm_hwm_kib": final_vm_hwm_kib,
        "delta_vm_hwm_kib": delta_vm_hwm_kib,
        "ceiling_vm_hwm_kib": MAX_DELTA_KIB,
        "elapsed_seconds": elapsed_seconds,
        "throughput_mib_s": throughput_mib_s,
    });
    println!("{METRIC_PREFIX}{metric}");
    std::io::stdout()
        .flush()
        .expect("RSS metric stdout flush failed");

    assert_eq!(written, TOTAL_BYTES);
    assert!(
        delta_vm_hwm_kib <= MAX_DELTA_KIB,
        "100 MiB download increased child VmHWM by {delta_vm_hwm_kib} KiB; ceiling is {MAX_DELTA_KIB} KiB"
    );
}

async fn run_server_parent() {
    let pattern = test_pattern();
    // This represents a 100 MiB wire body while all 1600 scripted frames share
    // one immutable 64 KiB allocation in the parent process. The isolated
    // child's VmHWM therefore measures only the download path.
    let chunks = vec![Bytes::copy_from_slice(&pattern); TOTAL_BYTES / CHUNK_BYTES];
    assert_eq!(chunks.iter().map(Bytes::len).sum::<usize>(), TOTAL_BYTES);

    let server = TestServer::start(vec![ScriptedResponse::chunked(
        200,
        "application/octet-stream",
        chunks,
    )])
    .await;
    let directory = tempfile::tempdir().expect("RSS gate tempdir creation failed");
    let destination = directory.path().join("download.bin");

    let stdout_path = directory.path().join("child.stdout");
    let stderr_path = directory.path().join("child.stderr");
    let stdout_file = std::fs::File::create(&stdout_path).expect("could not create child stdout");
    let stderr_file = std::fs::File::create(&stderr_path).expect("could not create child stderr");
    let mut command = Command::new(
        std::env::current_exe().expect("could not locate the RSS integration-test binary"),
    );
    command
        .arg("--ignored")
        .arg("--exact")
        .arg(TEST_NAME)
        .arg("--nocapture")
        .arg("--test-threads=1")
        .env(CHILD_MARKER, "1")
        .env(BASE_URL_ENV, &server.base_url)
        .env(DESTINATION_ENV, &destination)
        .stdin(Stdio::null())
        .stdout(Stdio::from(stdout_file))
        .stderr(Stdio::from(stderr_file));

    // Async try_wait polling leaves this runtime free to serve the response.
    // If the parent test is cancelled or unwinds, the guard kills and reaps
    // the child before TempDir removes the destination and any private partial.
    let mut child = ChildGuard::new(
        command
            .spawn()
            .expect("could not run the isolated RSS child"),
    );
    let status = tokio::time::timeout(CHILD_LIVENESS_TIMEOUT, async {
        loop {
            if let Some(status) = child
                .try_wait()
                .expect("could not query the isolated RSS child")
            {
                break status;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("RSS child exceeded its five-minute liveness watchdog");
    child.disarm();
    server.shutdown().await;

    let stdout = std::fs::read_to_string(stdout_path).expect("could not read child stdout");
    let stderr = std::fs::read_to_string(stderr_path).expect("could not read child stderr");
    assert!(
        status.success(),
        "RSS child failed\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
    let metric = parse_metric(&stdout);
    println!("{METRIC_PREFIX}{metric}");

    assert_eq!(metric["schema"], "zai-rs.file-download-rss.v1");
    assert_eq!(metric_u64(&metric, "bytes"), TOTAL_BYTES as u64);
    let baseline = metric_u64(&metric, "baseline_vm_hwm_kib");
    let final_hwm = metric_u64(&metric, "final_vm_hwm_kib");
    let delta = metric_u64(&metric, "delta_vm_hwm_kib");
    assert_eq!(final_hwm.checked_sub(baseline), Some(delta));
    assert_eq!(metric_u64(&metric, "ceiling_vm_hwm_kib"), MAX_DELTA_KIB);
    assert!(metric_f64(&metric, "elapsed_seconds") > 0.0);
    assert!(metric_f64(&metric, "throughput_mib_s") > 0.0);
    assert!(
        delta <= MAX_DELTA_KIB,
        "isolated child exceeded the VmHWM delta ceiling"
    );

    verify_file_with_fixed_buffer(&destination, &pattern);
    let partials = partial_files(directory.path());
    assert!(partials.is_empty(), "private partials remain: {partials:?}");

    let requests = server.requests();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].path, "/api/paas/v4/files/rss-gate/content");
}

struct ChildGuard(Option<Child>);

impl ChildGuard {
    fn new(child: Child) -> Self {
        Self(Some(child))
    }

    fn try_wait(&mut self) -> std::io::Result<Option<ExitStatus>> {
        self.0
            .as_mut()
            .expect("RSS child guard was already disarmed")
            .try_wait()
    }

    fn disarm(&mut self) {
        self.0.take();
    }
}

impl Drop for ChildGuard {
    fn drop(&mut self) {
        if let Some(child) = self.0.as_mut() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

fn read_vm_hwm_kib(pid: u32) -> u64 {
    let status = std::fs::read_to_string(format!("/proc/{pid}/status"))
        .expect("could not read child /proc status");
    let line = status
        .lines()
        .find(|line| line.starts_with("VmHWM:"))
        .expect("/proc status did not contain VmHWM");
    let mut fields = line.split_whitespace();
    assert_eq!(fields.next(), Some("VmHWM:"));
    let value = fields
        .next()
        .expect("VmHWM did not contain a value")
        .parse::<u64>()
        .expect("VmHWM value was not an integer");
    assert_eq!(fields.next(), Some("kB"));
    value
}

fn test_pattern() -> Vec<u8> {
    (0..CHUNK_BYTES)
        .map(|index| ((index * 31 + 17) % 251) as u8)
        .collect()
}

fn verify_file_with_fixed_buffer(destination: &Path, pattern: &[u8]) {
    let file = std::fs::File::open(destination).expect("published destination is missing");
    let mut reader = std::io::BufReader::with_capacity(CHUNK_BYTES, file);
    let mut buffer = [0_u8; CHUNK_BYTES];
    let mut total = 0_usize;

    loop {
        let read = reader
            .read(&mut buffer)
            .expect("could not verify the published destination");
        if read == 0 {
            break;
        }
        for (offset, actual) in buffer[..read].iter().enumerate() {
            let expected = pattern[(total + offset) % pattern.len()];
            assert_eq!(
                *actual,
                expected,
                "download content mismatch at byte {}",
                total + offset
            );
        }
        total += read;
    }

    assert_eq!(total, TOTAL_BYTES);
}

fn partial_files(directory: &Path) -> Vec<PathBuf> {
    std::fs::read_dir(directory)
        .expect("could not inspect the download directory")
        .map(|entry| entry.expect("invalid download directory entry").path())
        .filter(|path| {
            path.file_name()
                .is_some_and(|name| name.to_string_lossy().ends_with(".part"))
        })
        .collect()
}

fn parse_metric(stdout: &str) -> Value {
    let encoded = stdout
        .lines()
        .find_map(|line| line.split_once(METRIC_PREFIX).map(|(_, encoded)| encoded))
        .expect("RSS child did not emit its metric line");
    serde_json::Deserializer::from_str(encoded)
        .into_iter::<Value>()
        .next()
        .expect("RSS child metric was empty")
        .expect("RSS child metric was not valid JSON")
}

fn metric_u64(metric: &Value, field: &str) -> u64 {
    metric[field]
        .as_u64()
        .unwrap_or_else(|| panic!("RSS metric field {field} was not an unsigned integer"))
}

fn metric_f64(metric: &Value, field: &str) -> f64 {
    metric[field]
        .as_f64()
        .filter(|value| value.is_finite())
        .unwrap_or_else(|| panic!("RSS metric field {field} was not a finite number"))
}
