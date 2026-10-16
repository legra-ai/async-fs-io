use crate::{
    AsyncFile, DirectoryEntryKind, DirectoryReader, FsError, TempDir, atomic_write_string,
    canonicalize, read_bounded, read_string_bounded, read_string_bounded_if_exists, remove_file,
    set_permissions, symlink_metadata, try_exists,
};

async fn test_root() -> TempDir {
    TempDir::create(std::env::temp_dir())
        .await
        .expect("create async test root")
}

#[tokio::test]
async fn bounded_read_rejects_a_file_larger_than_the_declared_limit() {
    let root = test_root().await;
    let path = root.path().join("large.txt");
    let mut file = AsyncFile::create(&path).await.expect("create file");
    file.write_all(b"0123456789").await.expect("write file");
    file.flush().await.expect("flush file");

    let result = read_bounded(&path, 9).await;

    assert!(matches!(result, Err(FsError::InvalidRequest(_))));
    root.remove().await.expect("remove test root");
}

#[tokio::test]
async fn bounded_text_read_accepts_content_within_the_limit() {
    let root = test_root().await;
    let path = root.path().join("text.txt");
    atomic_write_string(&path, "hello")
        .await
        .expect("write text");

    assert_eq!(
        read_string_bounded(&path, 5).await.expect("read text"),
        "hello"
    );
    root.remove().await.expect("remove test root");
}

#[tokio::test]
async fn directory_reader_keeps_one_entry_at_a_time() {
    let root = test_root().await;
    let first = root.path().join("first");
    let second = root.path().join("second");
    AsyncFile::create(&first).await.expect("create first");
    AsyncFile::create(&second).await.expect("create second");

    let mut reader = DirectoryReader::open(root.path())
        .await
        .expect("open directory");
    let mut seen = 0usize;
    while let Some(entry) = reader.next().await.expect("read directory entry") {
        assert_eq!(entry.kind(), DirectoryEntryKind::File);
        seen += 1;
    }

    assert_eq!(seen, 2);
    root.remove().await.expect("remove test root");
}

#[tokio::test]
async fn missing_directory_and_file_are_distinguished_without_blocking_io() {
    let root = test_root().await;
    let missing_dir = root.path().join("missing-directory");
    assert!(
        DirectoryReader::open_if_exists(&missing_dir)
            .await
            .expect("inspect missing directory")
            .is_none()
    );
    assert!(
        !try_exists(&missing_dir)
            .await
            .expect("inspect missing path")
    );
    assert_eq!(
        read_string_bounded_if_exists(&root.path().join("missing.txt"), 32)
            .await
            .expect("read missing text"),
        None
    );
    root.remove().await.expect("remove test root");
}

#[tokio::test]
async fn symlink_metadata_is_available_through_the_async_boundary() {
    let root = test_root().await;
    let path = root.path().join("file.txt");
    AsyncFile::create(&path).await.expect("create file");
    assert!(
        symlink_metadata(&path)
            .await
            .expect("read metadata")
            .is_file()
    );
    root.remove().await.expect("remove test root");
}

#[tokio::test]
async fn canonicalize_and_set_permissions_use_async_filesystem_boundary() {
    let root = test_root().await;
    let path = root.path().join("nested");
    crate::ensure_dir(&path)
        .await
        .expect("create nested directory");

    let canonical = canonicalize(&path).await.expect("canonicalize path");
    assert!(canonical.is_absolute());
    assert_eq!(canonical.file_name(), Some(std::ffi::OsStr::new("nested")));

    let file = path.join("file.txt");
    AsyncFile::create(&file).await.expect("create file");
    let permissions = symlink_metadata(&file)
        .await
        .expect("read metadata")
        .permissions();
    set_permissions(&file, permissions)
        .await
        .expect("set permissions");
    root.remove().await.expect("remove test root");
}

#[tokio::test]
async fn removing_a_missing_file_is_an_error() {
    let root = test_root().await;
    let result = remove_file(root.path().join("missing.txt")).await;
    assert!(matches!(result, Err(FsError::Write { .. })));
    root.remove().await.expect("remove test root");
}

#[tokio::test]
async fn temporary_directory_cleanup_is_explicitly_async() {
    let root = test_root().await;
    let nested = TempDir::create(root.path())
        .await
        .expect("create nested root");
    let nested_path = nested.path().to_owned();
    nested.remove().await.expect("remove nested root");
    assert!(
        AsyncFile::open_if_exists(nested_path)
            .await
            .expect("inspect nested root")
            .is_none()
    );
    root.remove().await.expect("remove test root");
}
