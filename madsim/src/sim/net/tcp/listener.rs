use std::{fmt, io::Result};
use tracing::instrument;

use crate::net::{IpProtocol::Tcp, *};

/// A TCP socket server, listening for connections.
#[cfg_attr(docsrs, doc(cfg(madsim)))]
pub struct TcpListener {
    guard: Arc<BindGuard>,
    /// Incoming connections.
    rx: async_channel::Receiver<TcpStream>,
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
    #[instrument]
    pub async fn bind<A: ToSocketAddrs>(addr: A) -> Result<TcpListener> {
        // TODO: simulate backlog
        let (tx, rx) = async_channel::unbounded();
        let guard = BindGuard::bind(addr, Tcp, Arc::new(TcpListenerSocket { tx })).await?;

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
    #[instrument]
    pub async fn accept(&self) -> Result<(TcpStream, SocketAddr)> {
        self.guard.net.rand_delay().await?;

        let mut stream = (self.rx.recv().await)
            .map_err(|e| io::Error::new(io::ErrorKind::ConnectionReset, e))?;
        let peer_addr = stream.peer;
        trace!(?peer_addr, "accept tcp connection");

        stream.guard = Some(self.guard.clone());
        Ok((stream, peer_addr))
    }

    /// Returns the local socket address.
    pub fn local_addr(&self) -> Result<SocketAddr> {
        Ok(self.guard.addr)
    }
}

/// Socket registered in the [`Network`].
struct TcpListenerSocket {
    tx: async_channel::Sender<TcpStream>,
}

impl Socket for TcpListenerSocket {
    fn new_connection(
        &self,
        peer: SocketAddr,
        addr: SocketAddr,
        tx: PayloadSender,
        rx: PayloadReceiver,
    ) {
        let stream = TcpStream {
            guard: None,
            addr,
            peer,
            write_buf: Default::default(),
            read_buf: Default::default(),
            tx,
            rx: rx.into(),
        };
        let _ = self.tx.try_send(stream);
    }
}
