use std::fmt;
use std::io::Result;
use std::net::SocketAddr;
use tracing::instrument;

use super::{lookup_host, Endpoint, ToSocketAddrs};

/// A UDP socket.
pub struct UdpSocket {
    ep: Endpoint,
}

impl fmt::Debug for UdpSocket {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.debug_struct("UdpSocket")
            .field("addr", &self.ep.local_addr().unwrap())
            .finish()
    }
}

impl UdpSocket {
    /// This function will create a new UDP socket and attempt to bind it to the `addr` provided.
    #[instrument]
    pub async fn bind<A: ToSocketAddrs>(addr: A) -> Result<Self> {
        let ep = Endpoint::bind(addr).await?;
        Ok(UdpSocket { ep })
    }

    /// Connects the UDP socket setting the default destination for send() and limiting packets
    /// that are read via recv from the address specified in `addr`.
    #[instrument]
    pub async fn connect<A: ToSocketAddrs>(&self, addr: A) -> Result<()> {
        let addr = lookup_host(addr).await?.next().unwrap();
        *self.ep.peer.lock() = Some(addr);
        Ok(())
    }

    /// Returns the local socket address.
    pub fn local_addr(&self) -> Result<SocketAddr> {
        self.ep.local_addr()
    }

    /// Returns the socket address of the remote peer this socket was connected to.
    pub fn peer_addr(&self) -> Result<SocketAddr> {
        self.ep.peer_addr()
    }

    /// Sends data on the socket to the given address. On success, returns the number of bytes written.
    #[instrument]
    pub async fn send_to(&self, dst: impl ToSocketAddrs, buf: &[u8]) -> Result<()> {
        self.ep.send_to(dst, 0, buf).await
    }

    /// Receives a single datagram message on the socket.
    /// On success, returns the number of bytes read and the origin.
    #[instrument]
    pub async fn recv_from(&self, buf: &mut [u8]) -> Result<(usize, SocketAddr)> {
        self.ep.recv_from(0, buf).await
    }

    /// Sends data on the socket to the remote address that the socket is connected to.
    #[instrument]
    pub async fn send(&self, buf: &[u8]) -> Result<()> {
        self.ep.send(0, buf).await
    }

    /// Receives a single datagram message on the socket from the remote address to which it is connected.
    /// On success, returns the number of bytes read.
    #[instrument]
    pub async fn recv(&self, buf: &mut [u8]) -> Result<usize> {
        self.ep.recv(0, buf).await
    }
}
