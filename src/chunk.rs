//! Whole-operation chunk I/O: open, position, transfer, and close a file in
//! one blocking-pool job instead of one hop per call.

use std::io::{
    Read,
    Seek,
    SeekFrom,
    Write,
};
use std::path::{
    Path,
    PathBuf,
};

use crate::{
    FsError,
    Operation,
};

async fn blocking<T, F>(operation: Operation, path: PathBuf, job: F) -> Result<T, FsError>
where
    T: Send + 'static,
    F: FnOnce(&Path) -> std::io::Result<T> + Send + 'static,
{
    let task_path = path.clone();
    match tokio::task::spawn_blocking(move || job(&task_path)).await {
        Ok(result) => result.map_err(|error| FsError::io(operation, &path, error)),
        Err(error) => Err(FsError::io(operation, &path, error)),
    }
}

/// Create or truncate `path` and write `bytes` as its whole content.
///
/// # Errors
/// Returns [`FsError`] when the file cannot be created or written.
pub async fn create_with_bytes(path: impl AsRef<Path>, bytes: Vec<u8>) -> Result<(), FsError> {
    blocking(Operation::Write, path.as_ref().to_owned(), move |path| {
        std::fs::File::create(path)?.write_all(&bytes)
    })
    .await
}

/// Append `bytes` to an existing file in one job. The buffer is dropped once
/// the write returns.
///
/// # Errors
/// Returns [`FsError`] when the file cannot be opened or written.
pub async fn append_bytes(path: impl AsRef<Path>, bytes: Vec<u8>) -> Result<(), FsError> {
    blocking(Operation::Write, path.as_ref().to_owned(), move |path| {
        std::fs::OpenOptions::new()
            .append(true)
            .open(path)?
            .write_all(&bytes)
    })
    .await
}

/// Overwrite `bytes` at `offset` in an existing file without truncating it.
///
/// # Errors
/// Returns [`FsError`] when the file cannot be opened, positioned, or written.
pub async fn write_bytes_at(
    path: impl AsRef<Path>,
    offset: u64,
    bytes: Vec<u8>,
) -> Result<(), FsError> {
    blocking(Operation::Write, path.as_ref().to_owned(), move |path| {
        let mut file = std::fs::OpenOptions::new().write(true).open(path)?;
        file.seek(SeekFrom::Start(offset))?;
        file.write_all(&bytes)
    })
    .await
}

/// Read up to `max_len` bytes starting at `offset`, but at least `min_len`
/// (failing with an unexpected-EOF error when the file is shorter). The
/// returned buffer is allocated once at its final size.
///
/// # Errors
/// Returns [`FsError`] when the file cannot be opened, positioned, or read,
/// or holds fewer than `min_len` bytes past `offset`.
pub async fn read_bytes_at(
    path: impl AsRef<Path>,
    offset: u64,
    min_len: usize,
    max_len: usize,
) -> Result<Vec<u8>, FsError> {
    blocking(Operation::Read, path.as_ref().to_owned(), move |path| {
        let mut file = std::fs::File::open(path)?;
        let on_disk = file.metadata()?.len().saturating_sub(offset);
        let size = usize::try_from(on_disk)
            .map_or(max_len, |on_disk| on_disk.min(max_len))
            .max(min_len);
        file.seek(SeekFrom::Start(offset))?;
        let mut bytes = vec![0; size];
        file.read_exact(&mut bytes)?;
        Ok(bytes)
    })
    .await
}
