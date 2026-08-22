#![doc = include_str!("../README.md")]

//! Async-first filesystem primitives with explicit memory bounds.

mod async_file;
mod atomic;
mod directory;
mod error;
mod file;
mod temp;

pub use async_file::AsyncFile;
pub use atomic::{atomic_copy, atomic_write, atomic_write_string};
pub use directory::{
    DirectoryEntry, DirectoryEntryKind, DirectoryReader, ensure_dir, remove_dir_all,
};
pub use error::{FsError, Operation};
pub use file::{
    FileMetadata, metadata, read_bounded, read_string_bounded, remove_if_exists, rename,
    write_bytes,
};
pub use temp::{TempDir, TempFile};

#[cfg(test)]
mod tests;
