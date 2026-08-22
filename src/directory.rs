//! Bounded, pull-based asynchronous directory traversal.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use crate::{FsError, Operation};

/// The kind of an asynchronously inspected directory entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DirectoryEntryKind {
    /// A regular file.
    File,
    /// A directory.
    Directory,
    /// A symbolic link.
    Symlink,
    /// Another filesystem entry kind.
    Other,
}

/// One directory entry. A reader holds at most one entry in addition to the
/// operating system's own directory stream buffer.
#[derive(Debug, Clone)]
pub struct DirectoryEntry {
    name: OsString,
    path: PathBuf,
    kind: DirectoryEntryKind,
}

impl DirectoryEntry {
    /// Return the entry name without its parent path.
    #[must_use]
    pub fn name(&self) -> &OsString {
        &self.name
    }

    /// Return the complete entry path.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Return the entry kind.
    #[must_use]
    pub const fn kind(&self) -> DirectoryEntryKind {
        self.kind
    }
}

/// An asynchronous directory reader. Call [`Self::next`] repeatedly instead
/// of collecting the directory into memory.
pub struct DirectoryReader {
    root: PathBuf,
    inner: tokio::fs::ReadDir,
}

impl DirectoryReader {
    /// Open a directory for asynchronous traversal.
    pub async fn open(path: impl AsRef<Path>) -> Result<Self, FsError> {
        let root = path.as_ref().to_owned();
        let inner = tokio::fs::read_dir(&root)
            .await
            .map_err(|error| FsError::io(Operation::Directory, &root, error))?;
        Ok(Self { root, inner })
    }

    /// Read the next entry, returning `None` at end of directory.
    pub async fn next(&mut self) -> Result<Option<DirectoryEntry>, FsError> {
        let Some(entry) = self
            .inner
            .next_entry()
            .await
            .map_err(|error| FsError::io(Operation::Directory, &self.root, error))?
        else {
            return Ok(None);
        };
        let file_type = entry
            .file_type()
            .await
            .map_err(|error| FsError::io(Operation::Directory, &self.root, error))?;
        let kind = if file_type.is_file() {
            DirectoryEntryKind::File
        } else if file_type.is_dir() {
            DirectoryEntryKind::Directory
        } else if file_type.is_symlink() {
            DirectoryEntryKind::Symlink
        } else {
            DirectoryEntryKind::Other
        };
        Ok(Some(DirectoryEntry {
            name: entry.file_name(),
            path: entry.path(),
            kind,
        }))
    }
}

/// Create a directory and all missing parents.
pub async fn ensure_dir(path: impl AsRef<Path>) -> Result<(), FsError> {
    let path = path.as_ref();
    tokio::fs::create_dir_all(path)
        .await
        .map_err(|error| FsError::io(Operation::Directory, path, error))
}

/// Remove a directory tree if it exists.
pub async fn remove_dir_all(path: impl AsRef<Path>) -> Result<(), FsError> {
    let path = path.as_ref();
    match tokio::fs::remove_dir_all(path).await {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(FsError::io(Operation::Directory, path, error)),
    }
}
