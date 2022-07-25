use log::trace;
use std::{io, net::SocketAddr, sync::Arc};

use crate::{
    net::{lookup_host, network::Socket, NetSim, TcpStream, ToSocketAddrs},
    plugin,
    task::NodeId,
};

/// A TCP socket server, listening for connections.
#[cfg_attr(docsrs, doc(cfg(madsim)))]
pub struct TcpListener {
    net: Arc<NetSim>,
    node: NodeId,
    /// Local address.
    addr: SocketAddr,
    /// Incoming connections.
    rx: async_channel::Receiver<(TcpStream, SocketAddr)>,
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
        let net = plugin::simulator::<NetSim>();
        let node = plugin::node();

        let (tx, rx) = async_channel::unbounded();
        let socket = Arc::new(TcpListenerSocket { tx });

        // attempt to bind to each address
        let mut last_err = None;
        for addr in lookup_host(addr).await? {
            net.rand_delay().await?;
            match net.network.lock().unwrap().bind(node, addr, socket.clone()) {
                Ok(addr) => {
                    trace!("tcp listening on {}", addr);
                    return Ok(TcpListener {
                        net: net.clone(),
                        node,
                        addr,
                        rx,
                    });
                }
                Err(e) => last_err = Some(e),
            }
        }

        Err(last_err.unwrap_or_else(|| {
            io::Error::new(io::ErrorKind::AddrNotAvailable, "no available addr to bind")
        }))
    }

    /// Accepts a new incoming connection from this listener.
    ///
    /// This function will yield once a new TCP connection is established. When
    /// established, the corresponding [`TcpStream`] and the remote peer's
    /// address will be returned.
    ///
    /// [`TcpStream`]: struct@crate::net::TcpStream
    pub async fn accept(&self) -> io::Result<(TcpStream, SocketAddr)> {
        let sim = plugin::simulator::<NetSim>();
        sim.rand_delay().await?;

        let (stream, addr) = (self.rx.recv().await)
            .map_err(|e| io::Error::new(io::ErrorKind::ConnectionReset, e))?;
        trace!("accept tcp connection from {}", addr);
        Ok((stream, addr))
    }

    /// Returns the local socket address.
    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        Ok(self.addr)
    }
}

impl Drop for TcpListener {
    fn drop(&mut self) {
        // avoid panic on panicking
        if let Ok(mut network) = self.net.network.lock() {
            network.close(self.node, self.addr.port());
        }
    }
}

/// Socket registered in the [`Network`].
pub(super) struct TcpListenerSocket {
    pub tx: async_channel::Sender<(TcpStream, SocketAddr)>,
}

impl Socket for TcpListenerSocket {}
