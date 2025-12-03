use crate::net::TcpStream;
use bytes::BufMut;
use std::{
    io,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

/// Borrowed read half of a [`TcpStream`].
#[derive(Debug)]
pub struct ReadHalf<'a>(&'a TcpStream);

/// Borrowed write half of a [`TcpStream`].
#[derive(Debug)]
pub struct WriteHalf<'a>(&'a TcpStream);

pub(crate) fn split(stream: &mut TcpStream) -> (ReadHalf<'_>, WriteHalf<'_>) {
    (ReadHalf(&*stream), WriteHalf(&*stream))
}

impl ReadHalf<'_> {
    /// Tries to read data from the stream into the provided buffer, advancing
    /// the buffer's internal cursor, returning how many bytes were read.
    ///
    /// Receives any pending data from the socket but does not wait for new data
    /// to arrive. On success, returns the number of bytes read. Because
    /// `try_read_buf()` is non-blocking, the buffer does not have to be stored
    /// by the async task and can exist entirely on the stack.
    pub fn try_read_buf<B: BufMut>(&self, buf: &mut B) -> io::Result<usize> {
        self.0.try_read_buf(buf)
    }
}

impl AsyncRead for ReadHalf<'_> {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        self.0.poll_read_priv(cx, buf)
    }
}

impl AsyncWrite for WriteHalf<'_> {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, io::Error>> {
        self.0.poll_write_priv(cx, buf)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), io::Error>> {
        self.0.poll_flush_priv(cx)
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), io::Error>> {
        self.0.poll_shutdown_priv(cx)
    }
}

impl AsRef<TcpStream> for ReadHalf<'_> {
    fn as_ref(&self) -> &TcpStream {
        self.0
    }
}

impl AsRef<TcpStream> for WriteHalf<'_> {
    fn as_ref(&self) -> &TcpStream {
        self.0
    }
}

/// Owned read half of a [`TcpStream`].
#[derive(Debug)]
pub struct OwnedReadHalf(Arc<TcpStream>);

/// Owned write half of a [`TcpStream`].
#[derive(Debug)]
pub struct OwnedWriteHalf(Arc<TcpStream>);

pub(crate) fn split_owned(stream: TcpStream) -> (OwnedReadHalf, OwnedWriteHalf) {
    let arc = Arc::new(stream);
    let read = OwnedReadHalf(Arc::clone(&arc));
    let write = OwnedWriteHalf(arc);
    (read, write)
}

impl OwnedReadHalf {
    /// Tries to read data from the stream into the provided buffer, advancing
    /// the buffer's internal cursor, returning how many bytes were read.
    ///
    /// Receives any pending data from the socket but does not wait for new data
    /// to arrive. On success, returns the number of bytes read. Because
    /// `try_read_buf()` is non-blocking, the buffer does not have to be stored
    /// by the async task and can exist entirely on the stack.
    pub fn try_read_buf<B: BufMut>(&self, buf: &mut B) -> io::Result<usize> {
        self.0.try_read_buf(buf)
    }
}

impl AsyncRead for OwnedReadHalf {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        self.0.poll_read_priv(cx, buf)
    }
}

impl AsyncWrite for OwnedWriteHalf {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, io::Error>> {
        self.0.poll_write_priv(cx, buf)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), io::Error>> {
        self.0.poll_flush_priv(cx)
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), io::Error>> {
        self.0.poll_shutdown_priv(cx)
    }
}

impl AsRef<TcpStream> for OwnedReadHalf {
    fn as_ref(&self) -> &TcpStream {
        &self.0
    }
}

impl AsRef<TcpStream> for OwnedWriteHalf {
    fn as_ref(&self) -> &TcpStream {
        &self.0
    }
}
