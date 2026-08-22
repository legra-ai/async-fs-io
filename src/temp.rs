//! Explicitly asynchronous temporary files and directories.

use std::path::{Path, PathBuf};

use crate::{AsyncFile, FsError, Operation};

/// An asynchronously created temporary directory.
pub struct TempDir {
    path: PathBuf,
}

impl TempDir {
    /// Create a uniquely named directory below `root`.
    pub async fn create(root: impl AsRef<Path>) -> Result<Self, FsError> {
        let root = root.as_ref().to_owned();
        tokio::fs::create_dir_all(&root)
            .await
            .map_err(|error| FsError::io(Operation::Directory, &root, error))?;
        let path = root.join(format!("tmp-{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir(&path)
            .await
            .map_err(|error| FsError::io(Operation::Directory, &path, error))?;
        Ok(Self { path })
    }

    /// Return the directory path.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Remove the directory and all its contents asynchronously.
    pub async fn remove(self) -> Result<(), FsError> {
        crate::directory::remove_dir_all(&self.path).await
    }
}

/// An asynchronously created temporary file with explicit cleanup.
pub struct TempFile {
    path: PathBuf,
    file: Option<AsyncFile>,
}

impl TempFile {
    /// Create a uniquely named temporary file below `root`.
    pub async fn create(root: impl AsRef<Path>) -> Result<Self, FsError> {
        let root = root.as_ref().to_owned();
        tokio::fs::create_dir_all(&root)
            .await
            .map_err(|error| FsError::io(Operation::Directory, &root, error))?;
        let path = root.join(format!("tmp-file-{}", uuid::Uuid::new_v4()));
        let file = AsyncFile::create_new(&path).await?;
        Ok(Self {
            path,
            file: Some(file),
        })
    }

    /// Return the temporary file path.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Borrow the open async file.
    pub fn file_mut(&mut self) -> Result<&mut AsyncFile, FsError> {
        self.file.as_mut().ok_or_else(|| {
            FsError::InvalidRequest("temporary file handle was already closed".to_owned())
        })
    }

    /// Close the handle and remove the file asynchronously.
    pub async fn remove(mut self) -> Result<(), FsError> {
        self.file.take();
        tokio::fs::remove_file(&self.path)
            .await
            .map_err(|error| FsError::io(Operation::Write, &self.path, error))
    }

    /// Close the handle while retaining the file on disk.
    pub fn persist(mut self) -> PathBuf {
        self.file.take();
        self.path
    }
}
