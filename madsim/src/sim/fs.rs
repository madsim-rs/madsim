//! Asynchronous file system.

use spin::{Mutex, RwLock};
use std::{
    collections::HashMap,
    fmt,
    io::{Error, ErrorKind, Result},
    path::{Path, PathBuf},
    sync::Arc,
};
use tracing::*;

use crate::{
    plugin::{node, simulator, Simulator},
    rand::GlobalRng,
    task::NodeId,
    time::TimeHandle,
    Config,
};

/// File system simulator.
#[cfg_attr(docsrs, doc(cfg(madsim)))]
#[derive(Default)]
pub struct FsSim {
    handles: Mutex<HashMap<NodeId, FsNodeHandle>>,
}

impl Simulator for FsSim {
    fn new(_rand: &GlobalRng, _time: &TimeHandle, _config: &Config) -> Self {
        Default::default()
    }

    fn create_node(&self, id: NodeId) {
        let mut handles = self.handles.lock();
        handles.insert(id, FsNodeHandle::new(id));
    }

    fn reset_node(&self, id: NodeId) {
        self.power_fail(id);
    }
}

impl FsSim {
    /// Return a handle of the specified node.
    fn get_node(&self, id: NodeId) -> FsNodeHandle {
        let handles = self.handles.lock();
        handles[&id].clone()
    }

    /// Simulate a power failure. All data that does not reach the disk will be lost.
    pub fn power_fail(&self, _id: NodeId) {
        // TODO
    }

    /// Get the size of given file.
    pub fn get_file_size(&self, node: NodeId, path: impl AsRef<Path>) -> Result<u64> {
        let path = path.as_ref();
        let handle = self.handles.lock()[&node].clone();
        let fs = handle.fs.lock();
        let inode = fs.get(path).ok_or_else(|| {
            Error::new(ErrorKind::NotFound, format!("file not found: {:?}", path))
        })?;
        Ok(inode.metadata().len())
    }
}

/// File system simulator for a node.
#[derive(Clone)]
struct FsNodeHandle {
    node: NodeId,
    fs: Arc<Mutex<HashMap<PathBuf, Arc<INode>>>>,
}

impl FsNodeHandle {
    fn new(node: NodeId) -> Self {
        trace!(?node, "create fs");
        FsNodeHandle {
            node,
            fs: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    fn current() -> Self {
        simulator::<FsSim>().get_node(node())
    }

    async fn open(&self, path: impl AsRef<Path>) -> Result<File> {
        let path = path.as_ref();
        trace!(node = ?self.node, ?path, "open file");
        let fs = self.fs.lock();
        let inode = fs
            .get(path)
            .ok_or_else(|| Error::new(ErrorKind::NotFound, format!("file not found: {:?}", path)))?
            .clone();
        Ok(File {
            inode,
            can_write: false,
        })
    }

    async fn create(&self, path: impl AsRef<Path>) -> Result<File> {
        let path = path.as_ref();
        trace!(node = ?self.node, ?path, "create file");
        let mut fs = self.fs.lock();
        let inode = fs
            .entry(path.into())
            .and_modify(|inode| inode.truncate())
            .or_insert_with(|| Arc::new(INode::new(path)))
            .clone();
        Ok(File {
            inode,
            can_write: true,
        })
    }

    async fn metadata(&self, path: impl AsRef<Path>) -> Result<Metadata> {
        let path = path.as_ref();
        let fs = self.fs.lock();
        let inode = fs.get(path).ok_or_else(|| {
            Error::new(ErrorKind::NotFound, format!("file not found: {:?}", path))
        })?;
        Ok(inode.metadata())
    }
}

struct INode {
    path: PathBuf,
    data: RwLock<Vec<u8>>,
}

impl INode {
    fn new(path: &Path) -> Self {
        INode {
            path: path.into(),
            data: RwLock::new(Vec::new()),
        }
    }

    fn truncate(&self) {
        self.data.write().clear();
    }

    fn metadata(&self) -> Metadata {
        Metadata {
            len: self.data.read().len() as u64,
        }
    }
}

/// A reference to an open file on the filesystem.
pub struct File {
    inode: Arc<INode>,
    can_write: bool,
}

impl fmt::Debug for File {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.debug_struct("File")
            .field("path", &self.inode.path)
            .finish()
    }
}

impl File {
    /// Attempts to open a file in read-only mode.
    pub async fn open(path: impl AsRef<Path>) -> Result<File> {
        let handle = FsNodeHandle::current();
        handle.open(path).await
    }

    /// Opens a file in write-only mode.
    ///
    /// This function will create a file if it does not exist, and will truncate it if it does.
    pub async fn create(path: impl AsRef<Path>) -> Result<File> {
        let handle = FsNodeHandle::current();
        handle.create(path).await
    }

    /// Reads a number of bytes starting from a given offset.
    #[instrument(skip(buf), fields(len = buf.len()))]
    pub async fn read_at(&self, buf: &mut [u8], offset: u64) -> Result<usize> {
        let data = self.inode.data.read();
        let end = data.len().min(offset as usize + buf.len());
        let len = end - offset as usize;
        buf[..len].copy_from_slice(&data[offset as usize..end]);
        // TODO: random delay
        Ok(len)
    }

    /// Attempts to write an entire buffer starting from a given offset.
    #[instrument(skip(buf), fields(len = buf.len()))]
    pub async fn write_all_at(&self, buf: &[u8], offset: u64) -> Result<()> {
        if !self.can_write {
            return Err(Error::new(
                ErrorKind::PermissionDenied,
                "the file is read only",
            ));
        }
        let mut data = self.inode.data.write();
        let end = data.len().min(offset as usize + buf.len());
        let len = end - offset as usize;
        data[offset as usize..end].copy_from_slice(&buf[..len]);
        if len < buf.len() {
            data.extend_from_slice(&buf[len..]);
        }
        // TODO: random delay
        // TODO: simulate buffer, write will not take effect until flush or close
        Ok(())
    }

    /// Truncates or extends the underlying file, updating the size of this file to become `size`.
    #[instrument]
    pub async fn set_len(&self, size: u64) -> Result<()> {
        let mut data = self.inode.data.write();
        data.resize(size as usize, 0);
        // TODO: random delay
        Ok(())
    }

    /// Attempts to sync all OS-internal metadata to disk.
    #[instrument]
    pub async fn sync_all(&self) -> Result<()> {
        // TODO: random delay
        Ok(())
    }

    /// Queries metadata about the underlying file.
    #[instrument]
    pub async fn metadata(&self) -> Result<Metadata> {
        Ok(self.inode.metadata())
    }
}

/// Read the entire contents of a file into a bytes vector.
pub async fn read(path: impl AsRef<Path>) -> Result<Vec<u8>> {
    let handle = FsNodeHandle::current();
    let file = handle.open(path).await?;
    let data = file.inode.data.read().clone();
    // TODO: random delay
    Ok(data)
}

/// Given a path, query the file system to get information about a file, directory, etc.
pub async fn metadata(path: impl AsRef<Path>) -> Result<Metadata> {
    let handle = FsNodeHandle::current();
    handle.metadata(path).await
}

/// Metadata information about a file.
pub struct Metadata {
    len: u64,
}

impl Metadata {
    /// Returns the size of the file, in bytes, this metadata is for.
    #[allow(clippy::len_without_is_empty)]
    pub fn len(&self) -> u64 {
        self.len
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::Runtime;

    #[test]
    fn create_open_read_write() {
        let runtime = Runtime::new();
        let node = runtime.create_node().build();
        let f = node.spawn(async move {
            assert_eq!(
                File::open("file").await.err().unwrap().kind(),
                ErrorKind::NotFound
            );
            let file = File::create("file").await.unwrap();
            file.write_all_at(b"hello", 0).await.unwrap();

            let mut buf = [0u8; 10];
            let read_len = file.read_at(&mut buf, 2).await.unwrap();
            assert_eq!(read_len, 3);
            assert_eq!(&buf[..3], b"llo");
            drop(file);

            // writing to a read-only file should be denied
            let rofile = File::open("file").await.unwrap();
            assert_eq!(
                rofile.write_all_at(b"gg", 0).await.err().unwrap().kind(),
                ErrorKind::PermissionDenied
            );

            // create should truncate existing file
            let file = File::create("file").await.unwrap();
            let read_len = file.read_at(&mut buf, 0).await.unwrap();
            assert_eq!(read_len, 0);
        });
        runtime.block_on(f).unwrap();
    }
}
