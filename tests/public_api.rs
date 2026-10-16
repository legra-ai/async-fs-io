//! Public-API integration test: atomic writes land whole and temporary
//! directories clean up, from the shipped crate.

use async_fs_io::{
    TempDir,
    atomic_write_string,
    read_string_bounded,
};

#[tokio::test]
async fn atomic_string_write_round_trips() {
    let root = tempfile::tempdir().expect("scratch root");
    let dir = TempDir::create(root.path()).await.expect("temp dir");
    let target = dir.path().join("config.toml");
    atomic_write_string(&target, "answer = 42\n")
        .await
        .expect("write");
    assert_eq!(
        read_string_bounded(&target, 1024).await.expect("read"),
        "answer = 42\n"
    );
    let path = dir.path().to_path_buf();
    dir.remove().await.expect("remove");
    assert!(!path.exists(), "temporary directory is removed explicitly");
}
