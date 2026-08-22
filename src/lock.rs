//! Async advisory file locking.

use std::path::Path;

use crate::FsError;

/// An acquired non-blocking exclusive advisory lock.
///
/// The operating-system lock is released when this value is dropped. The
/// descriptor is intentionally private so callers cannot reintroduce direct
/// synchronous filesystem access around the lock boundary.
#[derive(Debug)]
pub struct ExclusiveLock {
    _file: std::fs::File,
}

/// Acquire a non-blocking exclusive advisory lock on `path`.
///
/// The open and lock syscalls run on Tokio's blocking pool because the
/// portable advisory-lock APIs are synchronous OS primitives. The returned
/// guard keeps the descriptor alive until it is dropped.
pub async fn acquire_exclusive_lock(path: impl AsRef<Path>) -> Result<ExclusiveLock, FsError> {
    let path = path.as_ref().to_owned();
    let task_path = path.clone();
    tokio::task::spawn_blocking(move || acquire_blocking(&task_path))
        .await
        .map_err(|error| FsError::Lock {
            path: path.display().to_string(),
            detail: format!("lock task failed: {error}"),
        })?
}

fn acquire_blocking(path: &Path) -> Result<ExclusiveLock, FsError> {
    let file = open_lock_file(path).map_err(|error| FsError::Lock {
        path: path.display().to_string(),
        detail: format!("failed to open lock file: {error}"),
    })?;

    ensure_lock_file_permissions(path);
    fs2::FileExt::try_lock_exclusive(&file).map_err(|error| FsError::Lock {
        path: path.display().to_string(),
        detail: format!("already held by another process: {error}"),
    })?;
    Ok(ExclusiveLock { _file: file })
}

fn open_lock_file(path: &Path) -> Result<std::fs::File, std::io::Error> {
    let mut write_options = std::fs::OpenOptions::new();
    write_options
        .create(true)
        .truncate(false)
        .read(true)
        .write(true);
    #[cfg(unix)]
    std::os::unix::fs::OpenOptionsExt::mode(&mut write_options, 0o664);

    match write_options.open(path) {
        Ok(file) => Ok(file),
        Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => {
            let mut read_options = std::fs::OpenOptions::new();
            read_options.read(true).open(path)
        }
        Err(error) => Err(error),
    }
}

#[cfg(unix)]
fn ensure_lock_file_permissions(path: &Path) {
    use std::os::unix::fs::PermissionsExt;

    let permissions = std::fs::Permissions::from_mode(0o664);
    let _ = std::fs::set_permissions(path, permissions);
}

#[cfg(not(unix))]
fn ensure_lock_file_permissions(_path: &Path) {}
