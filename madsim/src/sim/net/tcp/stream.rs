use crate::{
    net::{lookup_host, network::Socket, BindGuard, IpProtocol::Tcp, NetSim, ToSocketAddrs},
    plugin,
    task::NodeId,
};
use bytes::{Buf, Bytes, BytesMut};
use log::*;
use std::{
    fmt, io,
    net::SocketAddr,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};
use tokio::{
    io::{AsyncRead, AsyncWrite, ReadBuf},
    sync::{mpsc, oneshot},
};

/// A TCP stream between a local and a remote socket.
pub struct TcpStream {
    pub(super) guard: Option<Arc<BindGuard>>,
    addr: SocketAddr,
    peer: SocketAddr,
    /// Buffer write data to be flushed.
    write_buf: BytesMut,
    read_buf: Bytes,
    tx: mpsc::UnboundedSender<Bytes>,
    rx: mpsc::UnboundedReceiver<Bytes>,
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
    pub async fn connect<A: ToSocketAddrs>(addr: A) -> io::Result<TcpStream> {
        let net = plugin::simulator::<NetSim>();
        let node = plugin::node();
        let mut last_err = None;

        for addr in lookup_host(addr).await? {
            // a relay before resolve an address
            net.rand_delay().await?;
            match Self::connect1(&net, node, addr).await {
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
    async fn connect1(net: &Arc<NetSim>, node: NodeId, addr: SocketAddr) -> io::Result<TcpStream> {
        trace!("connecting to {}", addr);
        // send a request to listener and wait for TcpStream
        // FIXME: the port it uses should not be exclusive
        let guard = BindGuard::bind("0.0.0.0:0", Tcp, Arc::new(TcpStreamSocket)).await?;
        let (tx, rx) = oneshot::channel::<TcpStream>();
        net.send(node, guard.addr.port(), addr, Tcp, Box::new((node, tx)))
            .await?;
        let mut stream = rx
            .await
            .map_err(|e| io::Error::new(io::ErrorKind::ConnectionRefused, e))?;
        stream.guard = Some(Arc::new(guard));
        Ok(stream)
    }

    /// Creates a pair of [`TcpStream`].
    pub(super) fn pair(
        node1: NodeId,
        node2: NodeId,
        addr1: SocketAddr,
        addr2: SocketAddr,
    ) -> (TcpStream, TcpStream) {
        trace!("new tcp connection {} <-> {}", addr1, addr2);
        let net = plugin::simulator::<NetSim>();
        let (tx1, rx1) = net.channel(node1, addr2, Tcp);
        let (tx2, rx2) = net.channel(node2, addr1, Tcp);
        let s1 = TcpStream {
            guard: None,
            addr: addr1,
            peer: addr2,
            write_buf: Default::default(),
            read_buf: Default::default(),
            tx: tx1,
            rx: rx2,
        };
        let s2 = TcpStream {
            guard: None,
            addr: addr2,
            peer: addr1,
            write_buf: Default::default(),
            read_buf: Default::default(),
            tx: tx2,
            rx: rx1,
        };
        (s1, s2)
    }

    /// Sets the value of the `TCP_NODELAY` option on this socket.
    pub fn set_nodelay(&self, _nodelay: bool) -> io::Result<()> {
        // TODO: simulate TCP_NODELAY
        Ok(())
    }

    /// Returns the socket address of the local half of this TCP connection.
    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        Ok(self.addr)
    }

    /// Returns the socket address of the remote peer of this TCP connection.
    pub fn peer_addr(&self) -> io::Result<SocketAddr> {
        Ok(self.peer)
    }
}

impl AsyncRead for TcpStream {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        // read the buffer if not empty
        if !self.read_buf.is_empty() {
            let len = self.read_buf.len().min(buf.remaining());
            buf.put_slice(&self.read_buf[..len]);
            self.read_buf.advance(len);
            return Poll::Ready(Ok(()));
        }
        // otherwise wait on channel
        match self.rx.poll_recv(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(Some(data)) => {
                self.read_buf = data;
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
    ) -> Poll<io::Result<usize>> {
        self.write_buf.extend_from_slice(buf);
        // TODO: simulate buffer full, partial write
        Poll::Ready(Ok(buf.len()))
    }

    fn poll_flush(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        // send data
        let data = self.write_buf.split().freeze();
        self.tx
            .send(data)
            .map_err(|e| io::Error::new(io::ErrorKind::ConnectionReset, e))?;
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<io::Result<()>> {
        // TODO: simulate shutdown
        Poll::Ready(Ok(()))
    }
}

/// Socket registered in the [`Network`].
struct TcpStreamSocket;

impl Socket for TcpStreamSocket {}
