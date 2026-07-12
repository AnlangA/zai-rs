//! Streaming and atomic-download tests.
use tempfile::tempdir;
use zai_rs::client::transport::download::atomic_download;

#[tokio::test]
async fn atomic_download_writes_and_no_part_residue() {
    let dir = tempdir().unwrap();
    let dest = dir.path().join("out.bin");
    let body = bytes::Bytes::from_static(b"hello download");
    atomic_download(&dest, body.clone()).await.unwrap();
    let read = tokio::fs::read(&dest).await.unwrap();
    assert_eq!(read, body.as_ref());
    // No .part residue.
    for e in std::fs::read_dir(dir.path()).unwrap() {
        let e = e.unwrap();
        assert!(
            !e.file_name().to_string_lossy().ends_with(".part"),
            "part file residue: {:?}",
            e.file_name()
        );
    }
}

#[tokio::test]
async fn atomic_download_refuses_existing_target() {
    let dir = tempdir().unwrap();
    let dest = dir.path().join("out.bin");
    tokio::fs::write(&dest, b"prior").await.unwrap();
    let r = atomic_download(&dest, bytes::Bytes::from_static(b"new")).await;
    assert!(r.is_err(), "existing target must be refused");
    let read = tokio::fs::read(&dest).await.unwrap();
    assert_eq!(read, b"prior");
}
