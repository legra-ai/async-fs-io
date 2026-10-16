//! Async low-level file handle.

use std::io::SeekFrom;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::task::{Context, Poll};

use tokio::io::{AsyncRead, AsyncReadExt, AsyncSeekExt, AsyncWrite, AsyncWriteExt, ReadBuf};

use crate::{FsError, Operation};

/// A file handle whose open, read, write, seek, flush, and metadata operations
/// are all asynchronous.
pub struct AsyncFile {
    inner: tokio::fs::File,
    path: PathBuf,
}

impl AsyncFile {
    /// Open an existing file for reading.
    pub async fn open(path: impl AsRef<Path>) -> Result<Self, FsError> {
        let path = path.as_ref().to_owned();
        let inner = tokio::fs::File::open(&path)
            .await
            .map_err(|error| FsError::io(Operation::Read, &path, error))?;
        Ok(Self { inner, path })
    }

    /// Open an existing file, returning `None` when it does not exist.
    pub async fn open_if_exists(path: impl AsRef<Path>) -> Result<Option<Self>, FsError> {
        let path = path.as_ref().to_owned();
        match tokio::fs::File::open(&path).await {
            Ok(inner) => Ok(Some(Self { inner, path })),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(FsError::io(Operation::Read, &path, error)),
        }
    }

    /// Create or truncate a file for writing.
    pub async fn create(path: impl AsRef<Path>) -> Result<Self, FsError> {
        let path = path.as_ref().to_owned();
        let inner = tokio::fs::File::create(&path)
            .await
            .map_err(|error| FsError::io(Operation::Write, &path, error))?;
        Ok(Self { inner, path })
    }

    /// Create a new file, failing if it already exists.
    pub async fn create_new(path: impl AsRef<Path>) -> Result<Self, FsError> {
        let path = path.as_ref().to_owned();
        let inner = tokio::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .await
            .map_err(|error| FsError::io(Operation::Write, &path, error))?;
        Ok(Self { inner, path })
    }

    /// Open a file for appending, creating it when needed.
    pub async fn open_append(path: impl AsRef<Path>) -> Result<Self, FsError> {
        let path = path.as_ref().to_owned();
        let inner = tokio::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .read(true)
            .open(&path)
            .await
            .map_err(|error| FsError::io(Operation::Write, &path, error))?;
        Ok(Self { inner, path })
    }

    /// Open a file for writing and position it at its current end.
    pub async fn open_write_at_end(path: impl AsRef<Path>) -> Result<Self, FsError> {
        let path = path.as_ref().to_owned();
        let mut inner = tokio::fs::OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .truncate(false)
            .open(&path)
            .await
            .map_err(|error| FsError::io(Operation::Write, &path, error))?;
        inner
            .seek(SeekFrom::End(0))
            .await
            .map_err(|error| FsError::io(Operation::Write, &path, error))?;
        Ok(Self { inner, path })
    }

    /// Read exactly the requested number of bytes.
    pub async fn read_exact(&mut self, buffer: &mut [u8]) -> Result<(), FsError> {
        self.inner
            .read_exact(buffer)
            .await
            .map(|_| ())
            .map_err(|error| FsError::io(Operation::Read, &self.path, error))
    }

    /// Write all bytes in `buffer`.
    pub async fn write_all(&mut self, buffer: &[u8]) -> Result<(), FsError> {
        self.inner
            .write_all(buffer)
            .await
            .map_err(|error| FsError::io(Operation::Write, &self.path, error))
    }

    /// Flush buffered bytes to the operating system.
    pub async fn flush(&mut self) -> Result<(), FsError> {
        self.inner
            .flush()
            .await
            .map_err(|error| FsError::io(Operation::Write, &self.path, error))
    }

    /// Flush file contents and metadata to stable storage.
    pub async fn sync_all(&self) -> Result<(), FsError> {
        self.inner
            .sync_all()
            .await
            .map_err(|error| FsError::io(Operation::Write, &self.path, error))
    }

    /// Flush file contents to stable storage using the platform's data-sync
    /// operation.
    pub async fn sync_data(&self) -> Result<(), FsError> {
        self.inner
            .sync_data()
            .await
            .map_err(|error| FsError::io(Operation::Write, &self.path, error))
    }

    /// Seek to a position and return the new offset.
    pub async fn seek(&mut self, position: SeekFrom) -> Result<u64, FsError> {
        self.inner
            .seek(position)
            .await
            .map_err(|error| FsError::io(Operation::Read, &self.path, error))
    }

    /// Return the current stream position.
    pub async fn stream_position(&mut self) -> Result<u64, FsError> {
        self.seek(SeekFrom::Current(0)).await
    }

    /// Return metadata for the open file.
    pub async fn metadata(&self) -> Result<std::fs::Metadata, FsError> {
        self.inner
            .metadata()
            .await
            .map_err(|error| FsError::io(Operation::Read, &self.path, error))
    }

    /// Change the file length.
    pub async fn set_len(&self, length: u64) -> Result<(), FsError> {
        self.inner
            .set_len(length)
            .await
            .map_err(|error| FsError::io(Operation::Write, &self.path, error))
    }

    /// Return the path used to open this file.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl AsyncRead for AsyncFile {
    fn poll_read(
        mut self: Pin<&mut Self>,
        context: &mut Context<'_>,
        buffer: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.inner).poll_read(context, buffer)
    }
}

impl AsyncWrite for AsyncFile {
    fn poll_write(
        mut self: Pin<&mut Self>,
        context: &mut Context<'_>,
        buffer: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        Pin::new(&mut self.inner).poll_write(context, buffer)
    }

    fn poll_flush(
        mut self: Pin<&mut Self>,
        context: &mut Context<'_>,
    ) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.inner).poll_flush(context)
    }

    fn poll_shutdown(
        mut self: Pin<&mut Self>,
        context: &mut Context<'_>,
    ) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.inner).poll_shutdown(context)
    }
}
