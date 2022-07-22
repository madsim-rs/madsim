use super::TcpListenerSocket;
use crate::{
    net::{lookup_host, network::Socket, NetSim, ToSocketAddrs},
    plugin,
    task::NodeId,
};
use log::*;
use std::{
    fmt,
    future::Future,
    io,
    net::SocketAddr,
    pin::Pin,
    sync::{Arc, Weak},
    task::{Context, Poll},
    time::Duration,
};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

/// A TCP stream between a local and a remote socket.
pub struct TcpStream {
    net: Arc<NetSim>,
    node: NodeId,
    addr: SocketAddr,
    peer: SocketAddr,
    /// Buffer write data to be flushed.
    write_buf: Vec<u8>,
    /// Data that can be read.
    data: async_ringbuffer::Duplex,
    /// To indicate connection closed.
    peer_socket: Weak<TcpStreamSocket>,
}

// TODO: make sure it is safe
unsafe impl Send for TcpStream {}

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
            io::Error::new(io::ErrorKind::AddrNotAvailable, "no available addr to bind")
        }))
    }

    /// Connects to one address.
    async fn connect1(net: &Arc<NetSim>, node: NodeId, addr: SocketAddr) -> io::Result<TcpStream> {
        trace!("connecting to {}", addr);
        let (socket, latency) = (net.network.lock().unwrap())
            .try_send(node, addr)
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::AddrNotAvailable,
                    "there is no remote listener",
                )
            })?;
        let listener = socket.downcast_arc::<TcpListenerSocket>().map_err(|_| {
            io::Error::new(
                io::ErrorKind::AddrNotAvailable,
                "destination is not a TCP socket",
            )
        })?;
        // delay for 1.5 RTT handshake
        net.time.sleep(latency * 3).await;

        // bind sockets
        let unspecified = SocketAddr::from(([0, 0, 0, 0], 0));
        let dst_node = (net.network.lock().unwrap())
            .resolve_dest_node(node, addr)
            .unwrap();
        let local_socket = Arc::new(TcpStreamSocket);
        let peer_socket = Arc::new(TcpStreamSocket);
        let local_addr =
            (net.network.lock().unwrap()).bind(node, unspecified, local_socket.clone())?;
        let peer_addr =
            (net.network.lock().unwrap()).bind(dst_node, unspecified, peer_socket.clone())?;
        let (d1, d2) = async_ringbuffer::Duplex::pair(0x10_0000);
        let local = TcpStream {
            net: net.clone(),
            node,
            addr: local_addr,
            peer: peer_addr,
            write_buf: vec![],
            data: d1,
            peer_socket: Arc::downgrade(&peer_socket),
        };
        let peer = TcpStream {
            net: net.clone(),
            node: dst_node,
            addr: peer_addr,
            peer: local_addr,
            write_buf: vec![],
            data: d2,
            peer_socket: Arc::downgrade(&local_socket),
        };
        listener.tx.try_send((peer, local_addr)).unwrap();
        Ok(local)
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

impl Drop for TcpStream {
    fn drop(&mut self) {
        // avoid panic on panicking
        if let Ok(mut network) = self.net.network.lock() {
            network.close(self.node, self.addr.port());
        }
    }
}

impl AsyncRead for TcpStream {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        if self.peer_socket.strong_count() == 0 {
            return Poll::Ready(Err(io::Error::new(
                io::ErrorKind::ConnectionReset,
                "connection closed",
            )));
        }
        use tokio_util::compat::FuturesAsyncReadCompatExt;
        Pin::new(&mut (&mut self.data).compat()).poll_read(cx, buf)
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

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        // connection reset?
        if self.peer_socket.strong_count() == 0 {
            return Poll::Ready(Err(io::Error::new(
                io::ErrorKind::ConnectionReset,
                "connection closed",
            )));
        }
        // wait until link is available
        if (self.net.network.lock().unwrap())
            .try_send(self.node, self.peer)
            .is_none()
        {
            let mut sleep = self.net.time.sleep(Duration::from_secs(1));
            futures::ready!(Pin::new(&mut sleep).poll(cx));
        }
        // send data
        let this = self.get_mut();
        use tokio_util::compat::FuturesAsyncWriteCompatExt;
        let len = futures::ready!(
            Pin::new(&mut (&mut this.data).compat_write()).poll_write(cx, &this.write_buf)
        )?;
        this.write_buf.drain(..len);
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
