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
//! });
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
use spin::Mutex;
use std::{
    any::Any,
    io,
    net::{IpAddr, SocketAddr},
    sync::Arc,
};
use tokio::sync::{mpsc, oneshot};

use crate::{
    plugin,
    rand::{GlobalRng, Rng},
    task::{NodeId, TaskNodeHandle},
    time::{Duration, TimeHandle},
};

mod addr;
mod endpoint;
mod network;
#[cfg(feature = "rpc")]
#[cfg_attr(docsrs, doc(cfg(feature = "rpc")))]
pub mod rpc;
pub mod tcp;
mod udp;

pub use self::addr::{lookup_host, ToSocketAddrs};
pub use self::endpoint::Endpoint;
pub use self::network::{Config, Stat};
use self::network::{IpProtocol, Network, Socket};
pub use self::tcp::{TcpListener, TcpStream};
pub use self::udp::UdpSocket;

// #[cfg(unix)]
// pub mod unix;
// #[cfg(unix)]
// pub use unix::{UnixDatagram, UnixListener, UnixStream};

/// Network simulator.
#[cfg_attr(docsrs, doc(cfg(madsim)))]
pub struct NetSim {
    network: Mutex<Network>,
    rand: GlobalRng,
    time: TimeHandle,
    task: TaskNodeHandle,
}

/// Message sent to a network socket.
pub type Payload = Box<dyn Any + Send + Sync>;

impl plugin::Simulator for NetSim {
    fn new(_rand: &GlobalRng, _time: &TimeHandle, _config: &crate::Config) -> Self {
        unreachable!()
    }

    fn new1(
        rand: &GlobalRng,
        time: &TimeHandle,
        task: &TaskNodeHandle,
        config: &crate::Config,
    ) -> Self {
        NetSim {
            network: Mutex::new(Network::new(rand.clone(), config.net.clone())),
            rand: rand.clone(),
            time: time.clone(),
            task: task.clone(),
        }
    }

    fn create_node(&self, id: NodeId) {
        let mut network = self.network.lock();
        network.insert_node(id);
    }

    fn reset_node(&self, id: NodeId) {
        self.reset_node(id);
    }
}

impl NetSim {
    /// Get the statistics.
    pub fn stat(&self) -> Stat {
        self.network.lock().stat().clone()
    }

    /// Update network configurations.
    pub fn update_config(&self, f: impl FnOnce(&mut Config)) {
        let mut network = self.network.lock();
        network.update_config(f);
    }

    /// Reset a node.
    ///
    /// All connections will be closed.
    pub fn reset_node(&self, id: NodeId) {
        let mut network = self.network.lock();
        network.reset_node(id);
    }

    /// Set IP address of a node.
    pub fn set_ip(&self, node: NodeId, ip: IpAddr) {
        let mut network = self.network.lock();
        network.set_ip(node, ip);
    }

    /// Connect a node to the network.
    pub fn connect(&self, id: NodeId) {
        let mut network = self.network.lock();
        network.unclog_node(id);
    }

    /// Disconnect a node from the network.
    pub fn disconnect(&self, id: NodeId) {
        let mut network = self.network.lock();
        network.clog_node(id);
    }

    /// Connect a pair of nodes.
    pub fn connect2(&self, node1: NodeId, node2: NodeId) {
        let mut network = self.network.lock();
        network.unclog_link(node1, node2);
        network.unclog_link(node2, node1);
    }

    /// Disconnect a pair of nodes.
    pub fn disconnect2(&self, node1: NodeId, node2: NodeId) {
        let mut network = self.network.lock();
        network.clog_link(node1, node2);
        network.clog_link(node2, node1);
    }

    /// Delay a small random time and probably inject failure.
    async fn rand_delay(&self) -> io::Result<()> {
        let delay = Duration::from_micros(self.rand.with(|rng| rng.gen_range(0..5)));
        self.time.sleep(delay).await;
        // TODO: inject failure
        Ok(())
    }

    /// Send a message to the destination.
    pub(crate) async fn send(
        &self,
        node: NodeId,
        port: u16,
        dst: SocketAddr,
        protocol: IpProtocol,
        msg: Payload,
    ) -> io::Result<()> {
        if let Some((ip, socket, latency)) = self.network.lock().try_send(node, dst, protocol) {
            trace!("delay: {latency:?}");
            self.time.add_timer(latency, move || {
                socket.deliver((ip, port).into(), dst, msg);
            });
        };
        self.rand_delay().await?;
        Ok(())
    }

    /// Create a reliable, ordered channel between two endpoints.
    pub(crate) fn channel<T: Send + 'static>(
        self: &Arc<Self>,
        node: NodeId,
        dst: SocketAddr,
        protocol: IpProtocol,
    ) -> (mpsc::UnboundedSender<T>, mpsc::UnboundedReceiver<T>) {
        let (tx1, mut rx1) = mpsc::unbounded_channel::<T>();
        let (tx2, rx2) = mpsc::unbounded_channel::<T>();
        let net = self.clone();
        let handle = self.task.spawn(async move {
            while let Some(msg) = rx1.recv().await {
                // wait for link available
                let mut wait = Duration::from_millis(1);
                loop {
                    let res = net.network.lock().try_send(node, dst, protocol);
                    match res {
                        Some((_, _, latency)) => {
                            net.time.sleep(latency).await;
                            break;
                        }
                        None => {
                            net.time.sleep(wait).await;
                            // backoff
                            wait = (wait * 2).min(Duration::from_secs(10));
                        }
                    }
                }
                // receiver is closed. propagate the close to the sender.
                if tx2.send(msg).is_err() {
                    return;
                }
            }
            // sender is closed. propagate the close to the receiver.
        });
        self.network.lock().abort_task_on_reset(node, handle);
        (tx1, rx2)
    }
}

/// An RAII structure used to release the bound port.
pub(crate) struct BindGuard {
    net: Arc<NetSim>,
    node: NodeId,
    /// Bound address.
    addr: SocketAddr,
    protocol: IpProtocol,
}

impl BindGuard {
    /// Bind a socket to the address.
    pub async fn bind(
        addr: impl ToSocketAddrs,
        protocol: IpProtocol,
        socket: Arc<dyn Socket>,
    ) -> io::Result<Self> {
        let net = plugin::simulator::<NetSim>();
        let node = plugin::node();

        // attempt to bind to each address
        let mut last_err = None;
        for addr in lookup_host(addr).await? {
            net.rand_delay().await?;
            match net
                .network
                .lock()
                .bind(node, addr, protocol, socket.clone())
            {
                Ok(addr) => {
                    return Ok(BindGuard {
                        net: net.clone(),
                        node,
                        addr,
                        protocol,
                    })
                }
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
}

impl Drop for BindGuard {
    fn drop(&mut self) {
        // avoid panic on panicking
        if let Some(mut network) = self.net.network.try_lock() {
            network.close(self.node, self.addr, self.protocol);
        }
    }
}
