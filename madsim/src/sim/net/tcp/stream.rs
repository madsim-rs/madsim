use crate::net::{tcp::split, IpProtocol::Tcp, *};
use bytes::{Buf, BufMut, BytesMut};
use spin::Mutex;
#[cfg(unix)]
use std::os::unix::io::{AsRawFd, RawFd};
use std::{
    fmt,
    io::Result,
    pin::Pin,
    task::{Context, Poll},
};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tracing::*;

/// A TCP stream between a local and a remote socket.
pub struct TcpStream {
    pub(super) guard: Option<Arc<BindGuard>>,
    pub(super) addr: SocketAddr,
    pub(super) peer: SocketAddr,
    /// Buffer write data to be flushed.
    pub(super) write_buf: Mutex<BytesMut>,
    pub(super) read_buf: Mutex<Bytes>,
    pub(super) tx: PayloadSender,
    pub(super) rx: Mutex<PayloadReceiver>,
}

impl fmt::Debug for TcpStream {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.debug_struct("TcpStream")
            .field("addr", &self.addr)
            .field("peer", &self.peer)
            .finish()
    }
}

impl TcpStream {
    /// Opens a simulated TCP connection to a remote host.
    ///
    /// `addr` is an address of the remote host. Anything which implements the
    /// [`ToSocketAddrs`] trait can be supplied as the address.  If `addr`
    /// yields multiple addresses, connect will be attempted with each of the
    /// addresses until a connection is successful. If none of the addresses
    /// result in a successful connection, the error returned from the last
    /// connection attempt (the last address) is returned.
    ///
    /// [`ToSocketAddrs`]: trait@crate::net::ToSocketAddrs
    #[instrument]
    pub async fn connect<A: ToSocketAddrs>(addr: A) -> Result<TcpStream> {
        let mut last_err = None;

        for addr in lookup_host(addr).await? {
            match Self::connect_one(addr).await {
                Ok(stream) => return Ok(stream),
                Err(e) => last_err = Some(e),
            }
        }
        Err(last_err.unwrap_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "could not resolve to any addresses",
            )
        }))
    }

    /// Connects to one address.
    #[instrument]
    async fn connect_one(addr: SocketAddr) -> Result<TcpStream> {
        let net = plugin::simulator::<NetSim>();
        net.rand_delay().await?;

        // send a request to listener and wait for TcpStream
        // FIXME: the port it uses should not be exclusive
        let guard = BindGuard::bind("0.0.0.0:0", Tcp, Arc::new(TcpStreamSocket)).await?;
        let (tx, rx, local_addr) = net
            .connect1(plugin::node(), guard.addr.port(), addr, Tcp)
            .await?;
        let stream = TcpStream {
            guard: Some(Arc::new(guard)),
            addr: local_addr,
            peer: addr,
            write_buf: Default::default(),
            read_buf: Default::default(),
            tx,
            rx: Mutex::new(rx),
        };
        Ok(stream)
    }

    /// Sets the value of the `TCP_NODELAY` option on this socket.
    pub fn set_nodelay(&self, _nodelay: bool) -> Result<()> {
        // TODO: simulate TCP_NODELAY
        Ok(())
    }

    /// Returns the socket address of the local half of this TCP connection.
    pub fn local_addr(&self) -> Result<SocketAddr> {
        Ok(self.addr)
    }

    /// Returns the socket address of the remote peer of this TCP connection.
    pub fn peer_addr(&self) -> Result<SocketAddr> {
        Ok(self.peer)
    }

    /// Tries to read data from the stream into the provided buffer, advancing
    /// the buffer's internal cursor, returning how many bytes were read.
    ///
    /// Receives any pending data from the socket but does not wait for new data
    /// to arrive. On success, returns the number of bytes read. Because
    /// `try_read_buf()` is non-blocking, the buffer does not have to be stored
    /// by the async task and can exist entirely on the stack.
    pub fn try_read_buf<B: BufMut>(&self, buf: &mut B) -> io::Result<usize> {
        // read the buffer if not empty
        let mut read_buf = self.read_buf.lock();
        if !read_buf.is_empty() {
            let len = read_buf.len().min(buf.remaining_mut());
            buf.put_slice(&read_buf[..len]);
            read_buf.advance(len);
            return Ok(len);
        }
        Err(io::Error::new(
            io::ErrorKind::WouldBlock,
            "read buffer is empty",
        ))
    }

    /// Splits a `TcpStream` into a read half and a write half, which can be used
    /// to read and write the stream concurrently.
    pub fn split(&mut self) -> (split::ReadHalf<'_>, split::WriteHalf<'_>) {
        split::split(self)
    }

    /// Splits a `TcpStream` into a read half and a write half, which can be
    /// used to read and write the stream concurrently.
    pub fn into_split(self) -> (split::OwnedReadHalf, split::OwnedWriteHalf) {
        split::split_owned(self)
    }

    /// `poll_read` that takes `&self`.
    pub(super) fn poll_read_priv(
        &self,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<Result<()>> {
        // read the buffer if not empty
        if self.try_read_buf(buf).is_ok() {
            return Poll::Ready(Ok(()));
        }

        // otherwise wait on channel
        let poll_res = { self.rx.lock().poll_next_unpin(cx) };
        match poll_res {
            Poll::Pending => Poll::Pending,
            Poll::Ready(Some(data)) => {
                *self.read_buf.lock() = *data.downcast::<Bytes>().unwrap();
                self.poll_read_priv(cx, buf)
            }
            // ref: https://man7.org/linux/man-pages/man2/recv.2.html
            // > When a stream socket peer has performed an orderly shutdown, the
            // > return value will be 0 (the traditional "end-of-file" return).
            Poll::Ready(None) => Poll::Ready(Ok(())),
        }
    }

    /// `poll_write` that takes `&self`.
    pub(super) fn poll_write_priv(&self, _cx: &mut Context<'_>, buf: &[u8]) -> Poll<Result<usize>> {
        self.write_buf.lock().extend_from_slice(buf);
        // TODO: simulate buffer full, partial write
        Poll::Ready(Ok(buf.len()))
    }

    /// `poll_flush` that takes `&self`.
    pub(super) fn poll_flush_priv(&self, _cx: &mut Context<'_>) -> Poll<Result<()>> {
        // send data
        let data = self.write_buf.lock().split().freeze();
        self.tx
            .send(Box::new(data))
            .ok_or_else(|| io::Error::new(io::ErrorKind::ConnectionReset, "connection reset"))?;
        Poll::Ready(Ok(()))
    }

    /// `poll_shutdown` that takes `&self`.
    pub(super) fn poll_shutdown_priv(&self, _: &mut Context<'_>) -> Poll<Result<()>> {
        // TODO: simulate shutdown
        Poll::Ready(Ok(()))
    }
}

#[cfg(unix)]
impl AsRawFd for TcpStream {
    fn as_raw_fd(&self) -> RawFd {
        todo!("TcpStream::as_raw_fd");
    }
}

impl AsyncRead for TcpStream {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<Result<()>> {
        self.poll_read_priv(cx, buf)
    }
}

impl AsyncWrite for TcpStream {
    fn poll_write(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8]) -> Poll<Result<usize>> {
        self.poll_write_priv(cx, buf)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<()>> {
        self.poll_flush_priv(cx)
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<()>> {
        self.poll_shutdown_priv(cx)
    }
}

/// Socket registered in the [`Network`].
struct TcpStreamSocket;

impl Socket for TcpStreamSocket {}
