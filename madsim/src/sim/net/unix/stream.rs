use bytes::BufMut;
use std::{
    io::{self, Result},
    os::unix::{
        io::{AsRawFd, RawFd},
        net::SocketAddr,
    },
    path::Path,
    pin::Pin,
    task::{Context, Poll},
};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

/// A Unix socket which can accept connections from other Unix sockets.
#[doc(hidden)]
#[derive(Debug)]
pub struct UnixListener {}

impl UnixListener {
    /// Creates a new [`UnixListener`] bound to the specified path.
    pub fn bind<P>(path: P) -> Result<UnixListener>
    where
        P: AsRef<Path>,
    {
        todo!()
    }

    /// Accepts a new incoming connection to this listener.
    pub async fn accept(&self) -> Result<(UnixStream, SocketAddr)> {
        todo!()
    }
}

/// A structure representing a connected Unix socket.
#[doc(hidden)]
#[derive(Debug)]
pub struct UnixStream {}

impl UnixStream {
    /// Connects to the socket named by `path`.
    pub async fn connect<P>(path: P) -> Result<UnixStream>
    where
        P: AsRef<Path>,
    {
        todo!();
    }

    pub fn try_read_buf<B: BufMut>(&mut self, buf: &mut B) -> io::Result<usize> {
        unimplemented!();
    }
}

impl AsRawFd for UnixStream {
    fn as_raw_fd(&self) -> RawFd {
        todo!();
    }
}

impl AsyncRead for UnixStream {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<Result<()>> {
        todo!();
    }
}

impl AsyncWrite for UnixStream {
    fn poll_write(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize>> {
        todo!()
    }

    fn poll_flush(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<()>> {
        todo!()
    }

    fn poll_shutdown(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Result<()>> {
        todo!()
    }
}
