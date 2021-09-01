//! Asynchronous network endpoint and a controlled network simulator.

use log::*;
use std::{
    io,
    net::{SocketAddr, ToSocketAddrs},
    sync::{Arc, Mutex},
};

pub use self::network::{Config, Stat};
#[cfg(feature = "rpc")]
pub use self::rpc::Message;

mod network;
#[cfg(feature = "rpc")]
mod rpc;

/// Network handle to the runtime.
#[derive(Clone)]
pub struct NetHandle {}

impl NetHandle {
    /// Return a handle of the specified host.
    pub fn local_handle(&self, addr: SocketAddr) -> NetLocalHandle {
        todo!()
    }

    /// Get the statistics.
    pub fn stat(&self) -> Stat {
        Stat::default()
    }

    /// Update network configurations.
    pub fn update_config(&self, _f: impl FnOnce(&mut Config)) {}

    /// Connect a host to the network.
    pub fn connect(&self, addr: SocketAddr) {
        todo!()
    }

    /// Disconnect a host from the network.
    pub fn disconnect(&self, addr: SocketAddr) {
        todo!()
    }

    /// Connect a pair of hosts.
    pub fn connect2(&self, addr1: SocketAddr, addr2: SocketAddr) {
        todo!()
    }

    /// Disconnect a pair of hosts.
    pub fn disconnect2(&self, addr1: SocketAddr, addr2: SocketAddr) {
        todo!()
    }
}

/// Local host network handle to the runtime.
#[derive(Clone)]
pub struct NetLocalHandle {
    addr: SocketAddr,
}

impl NetLocalHandle {
    /// Returns a [`NetLocalHandle`] view over the currently running [`Runtime`].
    ///
    /// [`Runtime`]: crate::Runtime
    pub fn current() -> Self {
        Self { addr: todo!() }
    }

    /// Sends data with tag on the socket to the given address.
    ///
    /// # Example
    /// ```
    /// use madsim::{Runtime, net::NetLocalHandle};
    ///
    /// Runtime::new().block_on(async {
    ///     let net = NetLocalHandle::current();
    ///     net.send_to("127.0.0.1:4242", 0, &[0; 10]).await.expect("couldn't send data");
    /// });
    /// ```
    pub async fn send_to(&self, dst: impl ToSocketAddrs, tag: u64, data: &[u8]) -> io::Result<()> {
        let dst = dst.to_socket_addrs()?.next().unwrap();
        todo!()
    }

    /// Receives a single message with given tag on the socket.
    /// On success, returns the number of bytes read and the origin.
    ///
    /// # Example
    /// ```no_run
    /// use madsim::{Runtime, net::NetLocalHandle};
    ///
    /// Runtime::new().block_on(async {
    ///     let net = NetLocalHandle::current();
    ///     let mut buf = [0; 10];
    ///     let (len, src) = net.recv_from(0, &mut buf).await.expect("couldn't receive data");
    /// });
    /// ```
    pub async fn recv_from(&self, tag: u64, buf: &mut [u8]) -> io::Result<(usize, SocketAddr)> {
        let (len, from) = todo!();
        Ok((len, from))
    }
}
