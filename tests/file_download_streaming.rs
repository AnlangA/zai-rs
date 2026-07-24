//! File-content streaming, retry and atomic-publication contracts.

mod support;

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use bytes::Bytes;
use support::http_server::{ScriptedResponse, TestServer};
use zai_rs::client::{ApiFamily, HttpTransportConfig, ZaiClient};
use zai_rs::file::FileContentRequest;

const KEY: &str = "test.12345678901234567890";
const FILE_LIMIT: usize = 128 * 1024 * 1024;

fn client(server: &TestServer) -> ZaiClient {
    client_with_transport(server, HttpTransportConfig::default())
}

fn client_with_transport(server: &TestServer, transport: HttpTransportConfig) -> ZaiClient {
    ZaiClient::builder(KEY)
        .allow_insecure_transport(true)
        .endpoint(
            ApiFamily::PaasV4,
            format!("{}/api/paas/v4", server.base_url),
        )
        .transport(transport)
        .build()
        .unwrap()
}

fn partial_files(dir: &std::path::Path) -> Vec<std::path::PathBuf> {
    std::fs::read_dir(dir)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| {
            path.file_name()
                .is_some_and(|name| name.to_string_lossy().ends_with(".part"))
        })
        .collect()
}

#[tokio::test]
async fn multi_chunk_stream_buffer_and_atomic_file_paths_preserve_content() {
    let chunks = || {
        vec![
            Bytes::from_static(b"first-"),
            Bytes::from_static(b"second-"),
            Bytes::from_static(b"third"),
        ]
    };
    let server = TestServer::start(vec![
        ScriptedResponse::chunked(200, "application/octet-stream", chunks())
            .with_chunk_delay(Duration::from_millis(5)),
        ScriptedResponse::chunked(200, "application/octet-stream", chunks())
            .with_chunk_delay(Duration::from_millis(5)),
        ScriptedResponse::chunked(200, "application/octet-stream", chunks())
            .with_chunk_delay(Duration::from_millis(5)),
    ])
    .await;
    let client = client(&server);
    let expected = b"first-second-third";

    let mut stream = FileContentRequest::new("stream")
        .stream_via(&client)
        .await
        .unwrap();
    let mut streamed = Vec::new();
    let mut item_count = 0;
    while let Some(chunk) = stream.next().await {
        streamed.extend_from_slice(&chunk.unwrap());
        item_count += 1;
    }
    assert_eq!(streamed, expected);
    assert!(item_count >= 2, "the delayed response must remain chunked");

    let buffered = FileContentRequest::new("buffered")
        .send_via(&client)
        .await
        .unwrap();
    assert_eq!(buffered, expected);

    let dir = tempfile::tempdir().unwrap();
    let destination = dir.path().join("nested").join("download.bin");
    let written = FileContentRequest::new("file")
        .send_to_via(&client, &destination)
        .await
        .unwrap();
    assert_eq!(written, expected.len());
    assert_eq!(std::fs::read(&destination).unwrap(), expected);
    assert!(partial_files(destination.parent().unwrap()).is_empty());

    let requests = server.requests();
    assert_eq!(requests.len(), 3);
    assert_eq!(requests[0].path, "/api/paas/v4/files/stream/content");
    assert_eq!(requests[1].path, "/api/paas/v4/files/buffered/content");
    assert_eq!(requests[2].path, "/api/paas/v4/files/file/content");
    assert!(requests.iter().all(|request| {
        request
            .authorization
            .as_deref()
            .is_some_and(|value| value.starts_with("Bearer "))
    }));
    server.shutdown().await;
}

#[tokio::test]
async fn disconnect_before_first_chunk_retries_but_after_delivery_never_replays() {
    let before_first = TestServer::start(vec![
        ScriptedResponse::chunked(200, "application/octet-stream", Vec::<Bytes>::new())
            .disconnect_after(0),
        ScriptedResponse::chunked(
            200,
            "application/octet-stream",
            [Bytes::from_static(b"retried")],
        ),
    ])
    .await;
    let mut stream = FileContentRequest::new("retry")
        .stream_via(&client(&before_first))
        .await
        .unwrap();
    assert_eq!(
        stream.next().await.unwrap().unwrap(),
        Bytes::from_static(b"retried")
    );
    assert!(stream.next().await.is_none());
    assert_eq!(before_first.requests().len(), 2);
    before_first.shutdown().await;

    let after_first = TestServer::start(vec![
        ScriptedResponse::chunked(
            200,
            "application/octet-stream",
            [Bytes::from_static(b"visible")],
        )
        .with_chunk_delay(Duration::from_millis(5))
        .disconnect_after(1),
        ScriptedResponse::chunked(
            200,
            "application/octet-stream",
            [Bytes::from_static(b"must-not-be-requested")],
        ),
    ])
    .await;
    let mut stream = FileContentRequest::new("no-replay")
        .stream_via(&client(&after_first))
        .await
        .unwrap();
    assert_eq!(
        stream.next().await.unwrap().unwrap(),
        Bytes::from_static(b"visible")
    );
    assert!(stream.next().await.unwrap().is_err());
    assert!(stream.next().await.is_none());
    assert_eq!(
        after_first.requests().len(),
        1,
        "a visible body must permanently disable retries"
    );
    after_first.shutdown().await;
}

#[tokio::test]
async fn redirect_and_business_retry_keep_auth_and_file_contract() {
    let mut redirect = ScriptedResponse::empty(302);
    redirect.headers.push((
        "location".into(),
        "/api/paas/v4/files/redirected/content".into(),
    ));
    let server = TestServer::start(vec![
        redirect,
        ScriptedResponse::json(
            200,
            serde_json::json!({"code": 1200, "message": "retry me"}),
        ),
        ScriptedResponse::chunked(
            200,
            "application/octet-stream",
            [Bytes::from_static(b"complete")],
        ),
    ])
    .await;

    let body = FileContentRequest::new("original")
        .send_via(&client(&server))
        .await
        .unwrap();
    assert_eq!(body, b"complete");
    let requests = server.requests();
    assert_eq!(requests.len(), 3);
    assert_eq!(requests[0].path, "/api/paas/v4/files/original/content");
    assert_eq!(requests[1].path, "/api/paas/v4/files/redirected/content");
    assert_eq!(requests[2].path, "/api/paas/v4/files/redirected/content");
    assert!(requests.iter().all(|request| {
        request
            .authorization
            .as_deref()
            .is_some_and(|value| value.starts_with("Bearer "))
            && request
                .headers
                .iter()
                .any(|(name, value)| name == "accept" && value == "application/octet-stream")
    }));
    server.shutdown().await;
}

#[tokio::test]
async fn content_length_and_running_total_enforce_the_128_mib_limit() {
    let mut announced =
        ScriptedResponse::chunked(200, "application/octet-stream", [Bytes::from_static(b"x")])
            .with_chunk_delay(Duration::from_secs(1));
    announced
        .headers
        .push(("content-length".into(), (FILE_LIMIT + 1).to_string()));
    let announced_server = TestServer::start(vec![announced]).await;
    let error = match FileContentRequest::new("announced")
        .stream_via(&client(&announced_server))
        .await
    {
        Ok(_) => panic!("an oversized Content-Length must fail at the handshake"),
        Err(error) => error,
    };
    assert_eq!(
        error.code(),
        Some(zai_rs::client::error::codes::SDK_VALIDATION)
    );
    announced_server.shutdown().await;

    // Bytes clones share one 1-MiB allocation while the loopback server still
    // transmits 129 MiB, exercising the actual decoded running-total guard.
    let one_mib = Bytes::from(vec![0x5a; 1024 * 1024]);
    let running_server = TestServer::start(vec![ScriptedResponse::chunked(
        200,
        "application/octet-stream",
        vec![one_mib; 129],
    )])
    .await;
    let mut stream = FileContentRequest::new("running")
        .stream_via(&client(&running_server))
        .await
        .unwrap();
    let mut delivered = 0_usize;
    let error = loop {
        match stream.next().await {
            Some(Ok(chunk)) => delivered += chunk.len(),
            Some(Err(error)) => break error,
            None => panic!("an oversized stream ended without an error"),
        }
    };
    assert_eq!(delivered, FILE_LIMIT);
    assert_eq!(
        error.code(),
        Some(zai_rs::client::error::codes::SDK_VALIDATION)
    );
    assert!(stream.next().await.is_none());
    running_server.shutdown().await;
}

#[tokio::test]
async fn send_to_refuses_existing_target_and_cleans_failed_partial() {
    let existing_server = TestServer::start(Vec::new()).await;
    let existing_dir = tempfile::tempdir().unwrap();
    let existing = existing_dir.path().join("download.bin");
    std::fs::write(&existing, b"original").unwrap();
    FileContentRequest::new("existing")
        .send_to_via(&client(&existing_server), &existing)
        .await
        .unwrap_err();
    assert_eq!(std::fs::read(&existing).unwrap(), b"original");
    assert!(partial_files(existing_dir.path()).is_empty());
    assert!(
        existing_server.requests().is_empty(),
        "an existing target should fail before network I/O"
    );
    existing_server.shutdown().await;

    let failed_server = TestServer::start(vec![
        ScriptedResponse::chunked(
            200,
            "application/octet-stream",
            [Bytes::from_static(b"partial")],
        )
        .with_chunk_delay(Duration::from_millis(5))
        .disconnect_after(1),
    ])
    .await;
    let failed_dir = tempfile::tempdir().unwrap();
    let failed = failed_dir.path().join("download.bin");
    FileContentRequest::new("failed")
        .send_to_via(&client(&failed_server), &failed)
        .await
        .unwrap_err();
    assert!(!failed.exists());
    assert!(partial_files(failed_dir.path()).is_empty());
    assert_eq!(failed_server.requests().len(), 1);
    failed_server.shutdown().await;
}

#[tokio::test]
async fn cancellation_removes_partial_and_stream_waits_for_demanded_chunks() {
    let gate = Arc::new(tokio::sync::Semaphore::new(0));
    let emitted = Arc::new(AtomicUsize::new(0));
    let server = TestServer::start(vec![
        ScriptedResponse::chunked(
            200,
            "application/octet-stream",
            [Bytes::from_static(b"one"), Bytes::from_static(b"two")],
        )
        .with_chunk_gate(gate.clone())
        .with_chunk_counter(emitted.clone()),
    ])
    .await;
    let mut stream = FileContentRequest::new("backpressure")
        .stream_via(&client(&server))
        .await
        .unwrap();
    assert_eq!(emitted.load(Ordering::SeqCst), 0);
    assert!(
        tokio::time::timeout(Duration::from_millis(50), stream.next())
            .await
            .is_err(),
        "next must wait until the upstream produces a demanded chunk"
    );
    assert_eq!(emitted.load(Ordering::SeqCst), 0);
    gate.add_permits(1);
    assert_eq!(
        stream.next().await.unwrap().unwrap(),
        Bytes::from_static(b"one")
    );
    assert_eq!(emitted.load(Ordering::SeqCst), 1);
    gate.add_permits(1);
    assert_eq!(
        stream.next().await.unwrap().unwrap(),
        Bytes::from_static(b"two")
    );
    assert!(stream.next().await.is_none());
    server.shutdown().await;

    let cancel_gate = Arc::new(tokio::sync::Semaphore::new(0));
    let cancel_server = TestServer::start(vec![
        ScriptedResponse::chunked(
            200,
            "application/octet-stream",
            [Bytes::from_static(b"blocked")],
        )
        .with_chunk_gate(cancel_gate.clone()),
    ])
    .await;
    let cancel_client = client(&cancel_server);
    let cancel_dir = tempfile::tempdir().unwrap();
    let destination = cancel_dir.path().join("download.bin");
    let task_destination = destination.clone();
    let task = tokio::spawn(async move {
        FileContentRequest::new("cancel")
            .send_to_via(&cancel_client, task_destination)
            .await
    });

    tokio::time::timeout(Duration::from_secs(2), async {
        while partial_files(cancel_dir.path()).is_empty() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("the download never created its private partial file");
    task.abort();
    let _ = task.await;
    tokio::time::timeout(Duration::from_secs(2), async {
        while !partial_files(cancel_dir.path()).is_empty() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("cancellation left a .part file behind");
    assert!(!destination.exists());
    cancel_gate.close();
    cancel_server.shutdown().await;
}

#[tokio::test]
async fn consumer_pause_counts_against_the_absolute_attempt_deadline() {
    let gate = Arc::new(tokio::sync::Semaphore::new(1));
    let server = TestServer::start(vec![
        ScriptedResponse::chunked(
            200,
            "application/octet-stream",
            [Bytes::from_static(b"first"), Bytes::from_static(b"second")],
        )
        .with_chunk_gate(gate.clone()),
    ])
    .await;
    let transport = HttpTransportConfig::default()
        .with_request_timeout(Duration::from_millis(100))
        .unwrap()
        .with_max_attempts(1)
        .unwrap();
    let mut stream = FileContentRequest::new("deadline")
        .stream_via(&client_with_transport(&server, transport))
        .await
        .unwrap();
    assert_eq!(
        stream.next().await.unwrap().unwrap(),
        Bytes::from_static(b"first")
    );
    tokio::time::sleep(Duration::from_millis(150)).await;
    gate.add_permits(1);
    let error = stream.next().await.unwrap().unwrap_err();
    assert_eq!(
        error.code(),
        Some(zai_rs::client::error::codes::SDK_TIMEOUT)
    );
    assert!(stream.next().await.is_none());
    assert_eq!(server.requests().len(), 1);
    server.shutdown().await;
}
