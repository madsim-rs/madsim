//! Asynchronous network endpoint and a controlled network simulator.
//!
//! # Examples
//!
//! ```
//! use madsim::{runtime::Runtime, net::Endpoint};
//! use std::sync::Arc;
//! use std::net::SocketAddr;
//!
//! let runtime = Runtime::new();
//! let addr1 = "10.0.0.1:1".parse::<SocketAddr>().unwrap();
//! let addr2 = "10.0.0.2:1".parse::<SocketAddr>().unwrap();
//! let node1 = runtime.create_node().ip(addr1.ip()).build();
//! let node2 = runtime.create_node().ip(addr2.ip()).build();
//! let barrier = Arc::new(tokio::sync::Barrier::new(2));
//! let barrier_ = barrier.clone();
//!
//! node1.spawn(async move {
//!     let net = Endpoint::bind(addr1).await.unwrap();
//!     barrier_.wait().await;  // make sure addr2 has bound
//!
//!     net.send_to(addr2, 1, &[1]).await.unwrap();
//! })
//! .detach();
//!
//! let f = node2.spawn(async move {
//!     let net = Endpoint::bind(addr2).await.unwrap();
//!     barrier.wait().await;
//!
//!     let mut buf = vec![0; 0x10];
//!     let (len, from) = net.recv_from(1, &mut buf).await.unwrap();
//!     assert_eq!(from, addr1);
//!     assert_eq!(&buf[..len], &[1]);
//! });
//!
//! runtime.block_on(f);
//! ```

use log::*;
use std::{
    io,
    net::{IpAddr, Ipv4Addr, SocketAddr, ToSocketAddrs},
    sync::{Arc, Mutex},
};

pub use self::network::{Config, Stat};
use self::network::{Network, Payload};
use crate::{
    plugin,
    rand::{GlobalRng, Rng},
    task::NodeId,
    time::{Duration, TimeHandle},
};

mod network;
#[cfg(feature = "rpc")]
#[cfg_attr(docsrs, doc(cfg(feature = "rpc")))]
pub mod rpc;

/// Network simulator.
#[cfg_attr(docsrs, doc(cfg(feature = "sim")))]
pub struct NetSim {
    network: Mutex<Network>,
    rand: GlobalRng,
    time: TimeHandle,
}

impl plugin::Simulator for NetSim {
    fn new(rand: &GlobalRng, time: &TimeHandle, config: &crate::Config) -> Self {
        NetSim {
            network: Mutex::new(Network::new(rand.clone(), time.clone(), config.net.clone())),
            rand: rand.clone(),
            time: time.clone(),
        }
    }

    fn create_node(&self, id: NodeId) {
        let mut network = self.network.lock().unwrap();
        network.insert_node(id);
    }

    fn reset_node(&self, id: NodeId) {
        self.reset_node(id);
    }
}

impl NetSim {
    /// Get the statistics.
    pub fn stat(&self) -> Stat {
        self.network.lock().unwrap().stat().clone()
    }

    /// Update network configurations.
    pub fn update_config(&self, f: impl FnOnce(&mut Config)) {
        let mut network = self.network.lock().unwrap();
        network.update_config(f);
    }

    /// Reset a node.
    ///
    /// All connections will be closed.
    pub fn reset_node(&self, id: NodeId) {
        let mut network = self.network.lock().unwrap();
        network.reset_node(id);
    }

    /// Set IP address of a node.
    pub fn set_ip(&self, node: NodeId, ip: IpAddr) {
        let mut network = self.network.lock().unwrap();
        network.set_ip(node, ip);
    }

    /// Connect a node to the network.
    pub fn connect(&self, id: NodeId) {
        let mut network = self.network.lock().unwrap();
        network.unclog_node(id);
    }

    /// Disconnect a node from the network.
    pub fn disconnect(&self, id: NodeId) {
        let mut network = self.network.lock().unwrap();
        network.clog_node(id);
    }

    /// Connect a pair of nodes.
    pub fn connect2(&self, node1: NodeId, node2: NodeId) {
        let mut network = self.network.lock().unwrap();
        network.unclog_link(node1, node2);
        network.unclog_link(node2, node1);
    }

    /// Disconnect a pair of nodes.
    pub fn disconnect2(&self, node1: NodeId, node2: NodeId) {
        let mut network = self.network.lock().unwrap();
        network.clog_link(node1, node2);
        network.clog_link(node2, node1);
    }

    async fn rand_delay(&self) {
        let delay = Duration::from_micros(self.rand.with(|rng| rng.gen_range(0..5)));
        self.time.sleep(delay).await;
    }
}

/// An endpoint.
pub struct Endpoint {
    net: Arc<NetSim>,
    node: NodeId,
    addr: SocketAddr,
    peer: Option<SocketAddr>,
}

impl Endpoint {
    /// Creates a [`Endpoint`] from the given address.
    pub async fn bind(addr: impl ToSocketAddrs) -> io::Result<Self> {
        let net = plugin::simulator::<NetSim>();
        let node = plugin::node();
        let addr = addr.to_socket_addrs()?.next().unwrap();
        net.rand_delay().await;
        let addr = net.network.lock().unwrap().bind(node, addr)?;
        Ok(Endpoint {
            net,
            node,
            addr,
            peer: None,
        })
    }

    /// Connects this [`Endpoint`] to a remote address.
    pub async fn connect(addr: impl ToSocketAddrs) -> io::Result<Self> {
        let net = plugin::simulator::<NetSim>();
        let node = plugin::node();
        let peer = addr.to_socket_addrs()?.next().unwrap();
        net.rand_delay().await;
        let addr = if peer.ip().is_loopback() {
            SocketAddr::from((Ipv4Addr::LOCALHOST, 0))
        } else {
            SocketAddr::from((Ipv4Addr::UNSPECIFIED, 0))
        };
        let addr = net.network.lock().unwrap().bind(node, addr)?;
        Ok(Endpoint {
            net,
            node,
            addr,
            peer: Some(peer),
        })
    }

    /// Returns the local socket address.
    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        Ok(self.addr)
    }

    /// Returns the socket address of the remote peer this socket was connected to.
    pub fn peer_addr(&self) -> io::Result<SocketAddr> {
        self.peer
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotConnected, "not connected"))
    }

    /// Sends data with tag on the socket to the given address.
    ///
    /// # Example
    /// ```
    /// use madsim::{runtime::Runtime, net::Endpoint};
    ///
    /// Runtime::new().block_on(async {
    ///     let net = Endpoint::bind("127.0.0.1:0").await.unwrap();
    ///     net.send_to("127.0.0.1:4242", 0, &[0; 10]).await.expect("couldn't send data");
    /// });
    /// ```
    pub async fn send_to(&self, dst: impl ToSocketAddrs, tag: u64, buf: &[u8]) -> io::Result<()> {
        let dst = dst.to_socket_addrs()?.next().unwrap();
        self.send_to_raw(dst, tag, Box::new(Vec::from(buf))).await
    }

    /// Receives a single message with given tag on the socket.
    /// On success, returns the number of bytes read and the origin.
    ///
    /// # Example
    /// ```no_run
    /// use madsim::{runtime::Runtime, net::Endpoint};
    ///
    /// Runtime::new().block_on(async {
    ///     let net = Endpoint::bind("127.0.0.1:0").await.unwrap();
    ///     let mut buf = [0; 10];
    ///     let (len, src) = net.recv_from(0, &mut buf).await.expect("couldn't receive data");
    /// });
    /// ```
    pub async fn recv_from(&self, tag: u64, buf: &mut [u8]) -> io::Result<(usize, SocketAddr)> {
        let (data, from) = self.recv_from_raw(tag).await?;
        // copy to buffer
        let data = data.downcast::<Vec<u8>>().expect("message is not data");
        let len = buf.len().min(data.len());
        buf[..len].copy_from_slice(&data[..len]);
        Ok((len, from))
    }

    /// Sends data on the socket to the remote address to which it is connected.
    pub async fn send(&self, tag: u64, buf: &[u8]) -> io::Result<()> {
        let peer = self.peer_addr()?;
        self.send_to(peer, tag, buf).await
    }

    /// Receives a single datagram message on the socket from the remote address to which it is connected.
    /// On success, returns the number of bytes read.
    pub async fn recv(&self, tag: u64, buf: &mut [u8]) -> io::Result<usize> {
        let peer = self.peer_addr()?;
        let (len, from) = self.recv_from(tag, buf).await?;
        assert_eq!(
            from, peer,
            "receive a message but not from the connected address"
        );
        Ok(len)
    }

    /// Sends a raw message.
    ///
    /// NOTE: Applications should not use this function!
    /// It is provided for use by other simulators.
    #[cfg_attr(docsrs, doc(cfg(feature = "sim")))]
    pub async fn send_to_raw(&self, dst: SocketAddr, tag: u64, data: Payload) -> io::Result<()> {
        self.net
            .network
            .lock()
            .unwrap()
            .send(plugin::node(), self.addr, dst, tag, data);
        self.net.rand_delay().await;
        Ok(())
    }

    /// Receives a raw message.
    ///
    /// NOTE: Applications should not use this function!
    /// It is provided for use by other simulators.
    #[cfg_attr(docsrs, doc(cfg(feature = "sim")))]
    pub async fn recv_from_raw(&self, tag: u64) -> io::Result<(Payload, SocketAddr)> {
        let recver = self
            .net
            .network
            .lock()
            .unwrap()
            .recv(plugin::node(), self.addr, tag);
        let msg = recver
            .await
            .map_err(|_| io::Error::new(io::ErrorKind::BrokenPipe, "network is down"))?;
        self.net.rand_delay().await;

        trace!("recv: {} <- {}, tag={}", self.addr, msg.from, msg.tag);
        Ok((msg.data, msg.from))
    }

    /// Sends a raw message. to the connected remote address.
    ///
    /// NOTE: Applications should not use this function!
    /// It is provided for use by other simulators.
    #[cfg_attr(docsrs, doc(cfg(feature = "sim")))]
    pub async fn send_raw(&self, tag: u64, data: Payload) -> io::Result<()> {
        let peer = self.peer_addr()?;
        self.send_to_raw(peer, tag, data).await
    }

    /// Receives a raw message from the connected remote address.
    ///
    /// NOTE: Applications should not use this function!
    /// It is provided for use by other simulators.
    #[cfg_attr(docsrs, doc(cfg(feature = "sim")))]
    pub async fn recv_raw(&self, tag: u64) -> io::Result<Payload> {
        let peer = self.peer_addr()?;
        let (msg, from) = self.recv_from_raw(tag).await?;
        assert_eq!(
            from, peer,
            "receive a message but not from the connected address"
        );
        Ok(msg)
    }
}

impl Drop for Endpoint {
    fn drop(&mut self) {
        // avoid panic on panicking
        if let Ok(mut network) = self.net.network.lock() {
            network.close(self.node, self.addr);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{plugin::simulator, runtime::Runtime, time::*};
    use tokio::sync::Barrier;

    #[test]
    fn send_recv() {
        let runtime = Runtime::new();
        let addr1 = "10.0.0.1:1".parse::<SocketAddr>().unwrap();
        let addr2 = "10.0.0.2:1".parse::<SocketAddr>().unwrap();
        let node1 = runtime.create_node().ip(addr1.ip()).build();
        let node2 = runtime.create_node().ip(addr2.ip()).build();
        let barrier = Arc::new(Barrier::new(2));

        let barrier_ = barrier.clone();
        node1
            .spawn(async move {
                let net = Endpoint::bind(addr1).await.unwrap();
                barrier_.wait().await;

                net.send_to(addr2, 1, &[1]).await.unwrap();

                sleep(Duration::from_secs(1)).await;
                net.send_to(addr2, 2, &[2]).await.unwrap();
            })
            .detach();

        let f = node2.spawn(async move {
            let net = Endpoint::bind(addr2).await.unwrap();
            barrier.wait().await;

            let mut buf = vec![0; 0x10];
            let (len, from) = net.recv_from(2, &mut buf).await.unwrap();
            assert_eq!(len, 1);
            assert_eq!(from, addr1);
            assert_eq!(buf[0], 2);

            let (len, from) = net.recv_from(1, &mut buf).await.unwrap();
            assert_eq!(len, 1);
            assert_eq!(from, addr1);
            assert_eq!(buf[0], 1);
        });

        runtime.block_on(f);
    }

    #[test]
    fn receiver_drop() {
        let runtime = Runtime::new();
        let addr1 = "10.0.0.1:1".parse::<SocketAddr>().unwrap();
        let addr2 = "10.0.0.2:1".parse::<SocketAddr>().unwrap();
        let node1 = runtime.create_node().ip(addr1.ip()).build();
        let node2 = runtime.create_node().ip(addr2.ip()).build();
        let barrier = Arc::new(Barrier::new(2));

        let barrier_ = barrier.clone();
        node1
            .spawn(async move {
                let net = Endpoint::bind(addr1).await.unwrap();
                barrier_.wait().await;

                net.send_to(addr2, 1, &[1]).await.unwrap();
            })
            .detach();

        let f = node2.spawn(async move {
            let net = Endpoint::bind(addr2).await.unwrap();
            let mut buf = vec![0; 0x10];
            timeout(Duration::from_secs(1), net.recv_from(1, &mut buf))
                .await
                .err()
                .unwrap();
            // timeout and receiver dropped here
            barrier.wait().await;

            // receive again should success
            let (len, from) = net.recv_from(1, &mut buf).await.unwrap();
            assert_eq!(len, 1);
            assert_eq!(from, addr1);
        });

        runtime.block_on(f);
    }

    #[test]
    fn reset() {
        let runtime = Runtime::new();
        let addr1 = "10.0.0.1:1".parse::<SocketAddr>().unwrap();
        let node1 = runtime.create_node().ip(addr1.ip()).build();

        let f = node1.spawn(async move {
            let net = Endpoint::bind(addr1).await.unwrap();
            let err = net.recv_from(1, &mut []).await.unwrap_err();
            assert_eq!(err.kind(), std::io::ErrorKind::BrokenPipe);
            // FIXME: should still error
            // let err = net.recv_from(1, &mut []).await.unwrap_err();
            // assert_eq!(err.kind(), std::io::ErrorKind::BrokenPipe);
        });

        runtime.block_on(async move {
            sleep(Duration::from_secs(1)).await;
            simulator::<NetSim>().reset_node(node1.id());
            f.await;
        });
    }

    #[test]
    fn bind() {
        let runtime = Runtime::new();
        let ip = "10.0.0.1".parse::<IpAddr>().unwrap();
        let node = runtime.create_node().ip(ip).build();

        let f = node.spawn(async move {
            // unspecified
            let ep = Endpoint::bind("0.0.0.0:0").await.unwrap();
            let addr = ep.local_addr().unwrap();
            assert_eq!(addr.ip(), ip);
            assert_ne!(addr.port(), 0);

            // unspecified v6
            let ep = Endpoint::bind(":::0").await.unwrap();
            let addr = ep.local_addr().unwrap();
            assert_eq!(addr.ip(), ip);
            assert_ne!(addr.port(), 0);

            // localhost
            let ep = Endpoint::bind("127.0.0.1:0").await.unwrap();
            let addr = ep.local_addr().unwrap();
            assert_eq!(addr.ip().to_string(), "127.0.0.1");
            assert_ne!(addr.port(), 0);

            // localhost v6
            let ep = Endpoint::bind("::1:0").await.unwrap();
            let addr = ep.local_addr().unwrap();
            assert_eq!(addr.ip().to_string(), "::1");
            assert_ne!(addr.port(), 0);

            // wrong IP
            let err = Endpoint::bind("10.0.0.2:0").await.err().unwrap();
            assert_eq!(err.kind(), std::io::ErrorKind::AddrNotAvailable);

            // drop and reuse port
            let _ = Endpoint::bind("10.0.0.1:100").await.unwrap();
            let _ = Endpoint::bind("10.0.0.1:100").await.unwrap();
        });
        runtime.block_on(f);
    }

    #[test]
    #[ignore]
    fn localhost() {
        let runtime = Runtime::new();
        let ip1 = "10.0.0.1".parse::<IpAddr>().unwrap();
        let ip2 = "10.0.0.2".parse::<IpAddr>().unwrap();
        let node1 = runtime.create_node().ip(ip1).build();
        let node2 = runtime.create_node().ip(ip2).build();
        let barrier = Arc::new(Barrier::new(2));

        let barrier_ = barrier.clone();
        let f1 = node1.spawn(async move {
            let ep1 = Endpoint::bind("127.0.0.1:1").await.unwrap();
            let ep2 = Endpoint::bind("10.0.0.1:2").await.unwrap();
            barrier_.wait().await;

            // FIXME: ep1 should not receive messages from other node
            timeout(Duration::from_secs(1), ep1.recv_from(1, &mut []))
                .await
                .err()
                .expect("localhost endpoint should not receive from other nodes");
            // ep2 should receive
            ep2.recv_from(1, &mut []).await.unwrap();
        });
        let f2 = node2.spawn(async move {
            let ep = Endpoint::bind("127.0.0.1:1").await.unwrap();
            barrier.wait().await;

            ep.send_to("10.0.0.1:1", 1, &[1]).await.unwrap();
            ep.send_to("10.0.0.1:2", 1, &[1]).await.unwrap();
        });
        runtime.block_on(f1);
        runtime.block_on(f2);
    }

    #[test]
    fn connect_send_recv() {
        let runtime = Runtime::new();
        let addr1 = "10.0.0.1:1".parse::<SocketAddr>().unwrap();
        let addr2 = "10.0.0.2:1".parse::<SocketAddr>().unwrap();
        let node1 = runtime.create_node().ip(addr1.ip()).build();
        let node2 = runtime.create_node().ip(addr2.ip()).build();
        let barrier = Arc::new(Barrier::new(2));

        let barrier_ = barrier.clone();
        node1
            .spawn(async move {
                let ep = Endpoint::bind(addr1).await.unwrap();
                assert_eq!(ep.local_addr().unwrap(), addr1);
                barrier_.wait().await;

                let mut buf = vec![0; 0x10];
                let (len, from) = ep.recv_from(1, &mut buf).await.unwrap();
                assert_eq!(&buf[..len], b"ping");

                ep.send_to(from, 1, b"pong").await.unwrap();
            })
            .detach();

        let f = node2.spawn(async move {
            barrier.wait().await;
            let ep = Endpoint::connect(addr1).await.unwrap();
            assert_eq!(ep.peer_addr().unwrap(), addr1);

            ep.send(1, b"ping").await.unwrap();

            let mut buf = vec![0; 0x10];
            let len = ep.recv(1, &mut buf).await.unwrap();
            assert_eq!(&buf[..len], b"pong");
        });

        runtime.block_on(f);
    }
}
