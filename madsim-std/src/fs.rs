//! Asynchronous file system.

use std::{
    io::{Error, ErrorKind, Result},
    net::SocketAddr,
    path::{Path, PathBuf},
};

pub use tokio::fs::{metadata, read, File};

/// File system handle to the runtime.
#[derive(Clone)]
pub struct FsHandle {}

impl FsHandle {
    /// Simulate a power failure. All data that does not reach the disk will be lost.
    pub fn power_fail(&self, _addr: SocketAddr) {
        todo!()
    }

    /// Get the size of given file.
    pub fn get_file_size(&self, addr: SocketAddr, path: impl AsRef<Path>) -> Result<u64> {
        todo!()
    }
}
