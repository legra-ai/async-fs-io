//! Typed errors returned by the asynchronous filesystem boundary.

use std::path::Path;

/// The filesystem operation that failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Operation {
    /// Reading file contents or metadata.
    Read,
    /// Writing, renaming, or removing file contents.
    Write,
    /// Creating, traversing, or removing directories.
    Directory,
    /// Acquiring or releasing a filesystem lock.
    Lock,
    /// Resolving a path before I/O begins.
    PathResolution,
}

impl Operation {
    /// Return the stable lower-case operation name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Read => "read",
            Self::Write => "write",
            Self::Directory => "directory",
            Self::Lock => "lock",
            Self::PathResolution => "path resolution",
        }
    }
}

/// Error from an asynchronous filesystem operation.
#[derive(Debug, thiserror::Error)]
pub enum FsError {
    /// A file read operation failed.
    #[error("file read error: {path}: {detail}")]
    Read {
        /// The path involved in the failed operation.
        path: String,
        /// Human-readable operating-system detail.
        detail: String,
    },
    /// A file write operation failed.
    #[error("file write error: {path}: {detail}")]
    Write {
        /// The path involved in the failed operation.
        path: String,
        /// Human-readable operating-system detail.
        detail: String,
    },
    /// A directory operation failed.
    #[error("directory error: {path}: {detail}")]
    Directory {
        /// The directory involved in the failed operation.
        path: String,
        /// Human-readable operating-system detail.
        detail: String,
    },
    /// A filesystem lock operation failed.
    #[error("lock error: {path}: {detail}")]
    Lock {
        /// The lock path involved in the failed operation.
        path: String,
        /// Human-readable operating-system detail.
        detail: String,
    },
    /// A path could not be resolved before filesystem I/O began.
    #[error("path resolution error: {0}")]
    PathResolution(String),
    /// A caller supplied an invalid bound or path relationship.
    #[error("invalid filesystem request: {0}")]
    InvalidRequest(String),
}

impl FsError {
    /// Construct an error with the appropriate operation and path context.
    #[must_use]
    pub fn io(operation: Operation, path: &Path, detail: impl std::fmt::Display) -> Self {
        let path = path.display().to_string();
        let detail = detail.to_string();
        match operation {
            Operation::Read => Self::Read { path, detail },
            Operation::Write => Self::Write { path, detail },
            Operation::Directory => Self::Directory { path, detail },
            Operation::Lock => Self::Lock { path, detail },
            Operation::PathResolution => Self::PathResolution(detail),
        }
    }
}
