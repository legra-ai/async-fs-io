//! Bounded file convenience operations.

use std::path::{
    Path,
    PathBuf,
};

use tokio::io::AsyncReadExt;

use crate::{
    AsyncFile,
    FsError,
    Operation,
};

/// Metadata needed without exposing filesystem-specific read helpers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FileMetadata {
    /// File length in bytes.
    pub length: u64,
    /// Whether the path identifies a directory.
    pub is_directory: bool,
}

/// Canonicalize a path through the asynchronous filesystem boundary.
pub async fn canonicalize(path: impl AsRef<Path>) -> Result<PathBuf, FsError> {
    let path = path.as_ref();
    tokio::fs::canonicalize(path)
        .await
        .map_err(|error| FsError::io(Operation::Read, path, error))
}

/// Read a file only when its complete contents fit under `max_bytes`.
///
/// The implementation reads at most `max_bytes + 1` bytes and rejects a file
/// that exceeds the declared bound, so a stale metadata length cannot turn
/// this convenience API into an unbounded allocation.
pub async fn read_bounded(path: impl AsRef<Path>, max_bytes: usize) -> Result<Vec<u8>, FsError> {
    let path = path.as_ref();
    let file = AsyncFile::open(path).await?;
    let read_limit = max_bytes
        .checked_add(1)
        .ok_or_else(|| FsError::InvalidRequest("read bound overflow".to_owned()))?;
    // bounded: this allocation is capped by the caller-provided read limit.
    let mut output = Vec::with_capacity(max_bytes.min(64 * 1024));
    file.take(read_limit as u64)
        .read_to_end(&mut output)
        .await
        .map_err(|error| FsError::io(Operation::Read, path, error))?;
    if output.len() > max_bytes {
        return Err(FsError::InvalidRequest(format!(
            "file {} exceeds the {} byte read bound",
            path.display(),
            max_bytes
        )));
    }
    Ok(output)
}

/// Read UTF-8 text only when its complete contents fit under `max_bytes`.
pub async fn read_string_bounded(
    path: impl AsRef<Path>,
    max_bytes: usize,
) -> Result<String, FsError> {
    let path = path.as_ref();
    let bytes = read_bounded(path, max_bytes).await?;
    String::from_utf8(bytes).map_err(|error| {
        FsError::io(
            Operation::Read,
            path,
            format!("file is not valid UTF-8: {error}"),
        )
    })
}

/// Read UTF-8 text only when its complete contents fit under `max_bytes`,
/// returning `None` when the file does not exist.
pub async fn read_string_bounded_if_exists(
    path: impl AsRef<Path>,
    max_bytes: usize,
) -> Result<Option<String>, FsError> {
    let path = path.as_ref();
    let Some(file) = AsyncFile::open_if_exists(path).await? else {
        return Ok(None);
    };
    let read_limit = max_bytes
        .checked_add(1)
        .ok_or_else(|| FsError::InvalidRequest("read bound overflow".to_owned()))?;
    // bounded: this allocation is capped by the caller-provided read limit.
    let mut output = Vec::with_capacity(max_bytes.min(64 * 1024));
    file.take(read_limit as u64)
        .read_to_end(&mut output)
        .await
        .map_err(|error| FsError::io(Operation::Read, path, error))?;
    if output.len() > max_bytes {
        return Err(FsError::InvalidRequest(format!(
            "file {} exceeds the {} byte read bound",
            path.display(),
            max_bytes
        )));
    }
    String::from_utf8(output)
        .map(Some)
        .map_err(|error| FsError::io(Operation::Read, path, error))
}

/// Return metadata through the async filesystem boundary.
pub async fn metadata(path: impl AsRef<Path>) -> Result<FileMetadata, FsError> {
    let path = path.as_ref();
    let metadata = tokio::fs::metadata(path)
        .await
        .map_err(|error| FsError::io(Operation::Read, path, error))?;
    Ok(FileMetadata {
        length: metadata.len(),
        is_directory: metadata.is_dir(),
    })
}

/// Return operating-system metadata without following a symbolic link.
pub async fn symlink_metadata(path: impl AsRef<Path>) -> Result<std::fs::Metadata, FsError> {
    let path = path.as_ref();
    tokio::fs::symlink_metadata(path)
        .await
        .map_err(|error| FsError::io(Operation::Read, path, error))
}

/// Set filesystem permissions through the asynchronous filesystem boundary.
pub async fn set_permissions(
    path: impl AsRef<Path>,
    permissions: std::fs::Permissions,
) -> Result<(), FsError> {
    let path = path.as_ref();
    tokio::fs::set_permissions(path, permissions)
        .await
        .map_err(|error| FsError::io(Operation::Write, path, error))
}

/// Return whether a path exists, preserving errors other than not-found.
pub async fn try_exists(path: impl AsRef<Path>) -> Result<bool, FsError> {
    let path = path.as_ref();
    match tokio::fs::metadata(path).await {
        Ok(_) => Ok(true),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(FsError::io(Operation::Read, path, error)),
    }
}

/// Write a bounded caller-owned byte slice to a file, replacing its contents.
pub async fn write_bytes(path: impl AsRef<Path>, bytes: &[u8]) -> Result<(), FsError> {
    let path = path.as_ref();
    let mut file = AsyncFile::create(path).await?;
    file.write_all(bytes).await?;
    file.flush().await
}

/// Remove a file if it exists.
pub async fn remove_if_exists(path: impl AsRef<Path>) -> Result<(), FsError> {
    let path = path.as_ref();
    match tokio::fs::remove_file(path).await {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(FsError::io(Operation::Write, path, error)),
    }
}

/// Remove a file, reporting a missing file as an error.
pub async fn remove_file(path: impl AsRef<Path>) -> Result<(), FsError> {
    let path = path.as_ref();
    tokio::fs::remove_file(path)
        .await
        .map_err(|error| FsError::io(Operation::Write, path, error))
}

/// Rename a file or directory.
pub async fn rename(from: impl AsRef<Path>, to: impl AsRef<Path>) -> Result<(), FsError> {
    let from = from.as_ref();
    let to = to.as_ref();
    tokio::fs::rename(from, to).await.map_err(|error| {
        FsError::io(
            Operation::Write,
            to,
            format!("from {}: {error}", from.display()),
        )
    })
}
