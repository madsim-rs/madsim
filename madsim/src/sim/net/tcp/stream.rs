use crate::{
    net::{IpProtocol::Tcp, *},
    plugin,
};
use bytes::{Buf, Bytes, BytesMut};
use futures_util::StreamExt;
#[cfg(unix)]
use std::os::unix::io::{AsRawFd, RawFd};
use std::{
    fmt,
    io::Result,
    net::SocketAddr,
    pin::Pin,
    sync::Arc,
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
    pub(super) write_buf: BytesMut,
    pub(super) read_buf: Bytes,
    pub(super) tx: PayloadSender,
    pub(super) rx: PayloadReceiver,
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
            rx,
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
}

#[cfg(unix)]
impl AsRawFd for TcpStream {
    fn as_raw_fd(&self) -> RawFd {
        todo!("TcpStream::as_raw_fd");
    }
}

impl AsyncRead for TcpStream {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<Result<()>> {
        // read the buffer if not empty
        if !self.read_buf.is_empty() {
            let len = self.read_buf.len().min(buf.remaining());
            buf.put_slice(&self.read_buf[..len]);
            self.read_buf.advance(len);
            return Poll::Ready(Ok(()));
        }
        // otherwise wait on channel
        match self.rx.poll_next_unpin(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(Some(data)) => {
                self.read_buf = *data.downcast::<Bytes>().unwrap();
                self.poll_read(cx, buf)
            }
            // ref: https://man7.org/linux/man-pages/man2/recv.2.html
            // > When a stream socket peer has performed an orderly shutdown, the
            // > return value will be 0 (the traditional "end-of-file" return).
            Poll::Ready(None) => Poll::Ready(Ok(())),
        }
    }
}

impl AsyncWrite for TcpStream {
    fn poll_write(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize>> {
        self.write_buf.extend_from_slice(buf);
        // TODO: simulate buffer full, partial write
        Poll::Ready(Ok(buf.len()))
    }

    fn poll_flush(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<()>> {
        // send data
        let data = self.write_buf.split().freeze();
        self.tx
            .send(Box::new(data))
            .ok_or_else(|| io::Error::new(io::ErrorKind::ConnectionReset, "connection reset"))?;
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Result<()>> {
        // TODO: simulate shutdown
        Poll::Ready(Ok(()))
    }
}

/// Socket registered in the [`Network`].
struct TcpStreamSocket;

impl Socket for TcpStreamSocket {}
