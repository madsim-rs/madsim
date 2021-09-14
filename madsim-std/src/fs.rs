//! Asynchronous file system.

use std::{
    fs::Metadata,
    io::{Result, SeekFrom},
    net::SocketAddr,
    path::Path,
};

pub use tokio::fs::{metadata, read};
use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt};

/// File system handle to the runtime.
#[derive(Clone)]
pub struct FsHandle {}

impl FsHandle {
    pub(crate) fn new() -> Self {
        FsHandle {}
    }

    /// Simulate a power failure. All data that does not reach the disk will be lost.
    pub fn power_fail(&self, _addr: SocketAddr) {
        todo!()
    }

    /// Get the size of given file.
    pub fn get_file_size(&self, _addr: SocketAddr, _path: impl AsRef<Path>) -> Result<u64> {
        todo!()
    }
}

/// A reference to an open file on the filesystem.
pub struct File {
    inner: tokio::fs::File,
}

impl File {
    /// Attempts to open a file in read-only mode.
    pub async fn open(path: impl AsRef<Path>) -> Result<File> {
        Ok(File {
            inner: tokio::fs::File::open(path).await?,
        })
    }

    /// Opens a file in write-only mode.
    ///
    /// This function will create a file if it does not exist, and will truncate it if it does.
    pub async fn create(path: impl AsRef<Path>) -> Result<File> {
        Ok(File {
            inner: tokio::fs::File::create(path).await?,
        })
    }

    /// Reads a number of bytes starting from a given offset.
    pub async fn read_at(&mut self, buf: &mut [u8], offset: u64) -> Result<usize> {
        // TODO: make it &self
        self.inner.seek(SeekFrom::Start(offset)).await?;
        let len = self.inner.read(buf).await?;
        Ok(len)
    }

    /// Attempts to write an entire buffer starting from a given offset.
    pub async fn write_all_at(&mut self, buf: &[u8], offset: u64) -> Result<()> {
        // TODO: make it &self
        self.inner.seek(SeekFrom::Start(offset)).await?;
        let len = self.inner.write_all(buf).await?;
        Ok(len)
    }

    /// Truncates or extends the underlying file, updating the size of this file to become `size`.
    pub async fn set_len(&self, size: u64) -> Result<()> {
        self.inner.set_len(size).await
    }

    /// Attempts to sync all OS-internal metadata to disk.
    pub async fn sync_all(&self) -> Result<()> {
        self.inner.sync_all().await
    }

    /// Queries metadata about the underlying file.
    pub async fn metadata(&self) -> Result<Metadata> {
        self.inner.metadata().await
    }
}
