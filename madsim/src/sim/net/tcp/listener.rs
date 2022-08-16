use std::{fmt, io, net::SocketAddr, sync::Arc};
use tokio::sync::oneshot;

use crate::{
    net::{network::Socket, BindGuard, IpProtocol::Tcp, Payload, TcpStream, ToSocketAddrs},
    plugin,
    task::NodeId,
};

/// A TCP socket server, listening for connections.
#[cfg_attr(docsrs, doc(cfg(madsim)))]
pub struct TcpListener {
    guard: Arc<BindGuard>,
    /// Incoming connections.
    rx: async_channel::Receiver<(TcpStream, SocketAddr)>,
}

impl fmt::Debug for TcpListener {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.debug_struct("TcpListener")
            .field("addr", &self.guard.addr)
            .finish()
    }
}

impl TcpListener {
    /// Creates a new TcpListener, which will be bound to the specified address.
    ///
    /// The returned listener is ready for accepting connections.
    ///
    /// The address type can be any implementor of the [`ToSocketAddrs`] trait.
    /// If `addr` yields multiple addresses, bind will be attempted with each of
    /// the addresses until one succeeds and returns the listener. If none of
    /// the addresses succeed in creating a listener, the error returned from
    /// the last attempt (the last address) is returned.
    ///
    /// [`ToSocketAddrs`]: trait@crate::net::ToSocketAddrs
    pub async fn bind<A: ToSocketAddrs>(addr: A) -> io::Result<TcpListener> {
        // TODO: simulate backlog
        let (tx, rx) = async_channel::unbounded();
        let node = plugin::node();
        let guard = BindGuard::bind(addr, Tcp, Arc::new(TcpListenerSocket { tx, node })).await?;

        Ok(TcpListener {
            guard: Arc::new(guard),
            rx,
        })
    }

    /// Accepts a new incoming connection from this listener.
    ///
    /// This function will yield once a new TCP connection is established. When
    /// established, the corresponding [`TcpStream`] and the remote peer's
    /// address will be returned.
    ///
    /// [`TcpStream`]: struct@crate::net::TcpStream
    pub async fn accept(&self) -> io::Result<(TcpStream, SocketAddr)> {
        self.guard.net.rand_delay().await?;

        let (mut stream, addr) = (self.rx.recv().await)
            .map_err(|e| io::Error::new(io::ErrorKind::ConnectionReset, e))?;
        stream.guard = Some(self.guard.clone());
        Ok((stream, addr))
    }

    /// Returns the local socket address.
    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        Ok(self.guard.addr)
    }
}

/// Socket registered in the [`Network`].
struct TcpListenerSocket {
    tx: async_channel::Sender<(TcpStream, SocketAddr)>,
    node: NodeId,
}

impl Socket for TcpListenerSocket {
    fn deliver(&self, src: SocketAddr, dst: SocketAddr, msg: Payload) {
        let (src_node, tx): (NodeId, oneshot::Sender<TcpStream>) = *msg.downcast().unwrap();
        let (peer, local) = TcpStream::pair(src_node, self.node, src, dst);
        let _ = tx.send(peer);
        let _ = self.tx.try_send((local, src));
    }
}
