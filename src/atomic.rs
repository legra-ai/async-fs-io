//! Atomic, streaming file replacement.

use std::path::{
    Path,
    PathBuf,
};
use std::pin::Pin;
use std::task::{
    Context,
    Poll,
};

use tokio::io::{
    AsyncRead,
    ReadBuf,
};

use crate::{
    AsyncFile,
    FsError,
    Operation,
};

/// Stream `reader` into a temporary sibling file and atomically rename it over
/// `target`.
pub async fn atomic_write<R>(target: impl AsRef<Path>, mut reader: R) -> Result<(), FsError>
where
    R: AsyncRead + Unpin,
{
    let target = target.as_ref().to_owned();
    let parent = target.parent().ok_or_else(|| {
        FsError::InvalidRequest(format!("target has no parent: {}", target.display()))
    })?;
    tokio::fs::create_dir_all(parent)
        .await
        .map_err(|error| FsError::io(Operation::Directory, parent, error))?;

    let temporary = temporary_sibling(&target);
    let result = async {
        let mut file = AsyncFile::create_new(&temporary).await?;
        tokio::io::copy(&mut reader, &mut file)
            .await
            .map_err(|error| FsError::io(Operation::Write, &temporary, error))?;
        file.flush().await?;
        tokio::fs::rename(&temporary, &target)
            .await
            .map_err(|error| FsError::io(Operation::Write, &target, error))
    }
    .await;

    match result {
        Ok(()) => Ok(()),
        Err(primary) => {
            // Cleanup is part of the async failure path; it is not hidden in
            // Drop and a cleanup failure must remain visible to the caller.
            match tokio::fs::remove_file(&temporary).await {
                Ok(()) => Err(primary),
                Err(cleanup) => Err(FsError::Write {
                    path: temporary.display().to_string(),
                    detail: format!("{primary}; temporary-file cleanup failed: {cleanup}"),
                }),
            }
        }
    }
}

/// Atomically replace a file with UTF-8 text.
pub async fn atomic_write_string(target: impl AsRef<Path>, value: &str) -> Result<(), FsError> {
    atomic_write(target, SliceReader::new(value.as_bytes())).await
}

/// Atomically copy one existing file to another path.
pub async fn atomic_copy(
    source: impl AsRef<Path>,
    target: impl AsRef<Path>,
) -> Result<(), FsError> {
    let source = source.as_ref();
    let file = AsyncFile::open(source).await?;
    atomic_write(target, file).await
}

fn temporary_sibling(target: &Path) -> PathBuf {
    let mut value = target.as_os_str().to_owned();
    value.push(format!(".tmp.{}", uuid::Uuid::new_v4()));
    value.into()
}

struct SliceReader<'a> {
    bytes: &'a [u8],
    position: usize,
}

impl<'a> SliceReader<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, position: 0 }
    }
}

impl AsyncRead for SliceReader<'_> {
    fn poll_read(
        mut self: Pin<&mut Self>,
        _context: &mut Context<'_>,
        buffer: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        let remaining = &self.bytes[self.position..];
        if remaining.is_empty() {
            return Poll::Ready(Ok(()));
        }
        let amount = remaining.len().min(buffer.remaining());
        buffer.put_slice(&remaining[..amount]);
        self.position += amount;
        Poll::Ready(Ok(()))
    }
}
