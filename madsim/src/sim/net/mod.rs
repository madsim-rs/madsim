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

use bytes::Bytes;
use spin::Mutex;
use std::{
    any::Any,
    collections::HashMap,
    io,
    net::{IpAddr, SocketAddr},
    sync::Arc,
};
use tokio::sync::{mpsc, oneshot};
use tracing::*;

use crate::{
    plugin,
    rand::{GlobalRng, Rng},
    task::{NodeId, NodeInfo, Spawner},
    time::{Duration, TimeHandle},
};

mod addr;
mod dns;
mod endpoint;
mod network;
#[cfg(feature = "rpc")]
#[cfg_attr(docsrs, doc(cfg(feature = "rpc")))]
pub mod rpc;
pub mod tcp;
mod udp;
pub mod unix;

pub use self::addr::{lookup_host, ToSocketAddrs};
use self::dns::DnsServer;
pub use self::endpoint::{Endpoint, Receiver, Sender};
pub use self::network::{Config, Stat};
use self::network::{Direction, IpProtocol, Network, Socket};
pub use self::tcp::{TcpListener, TcpStream};
pub use self::udp::UdpSocket;
pub use self::unix::{UnixDatagram, UnixListener, UnixStream};

/// Network simulator.
#[cfg_attr(docsrs, doc(cfg(madsim)))]
pub struct NetSim {
    network: Mutex<Network>,
    dns: Mutex<DnsServer>,
    rand: GlobalRng,
    time: TimeHandle,
    task: Spawner,
    hooks_req: Mutex<HashMap<NodeId, MsgHookFn>>,
    hooks_rsp: Mutex<HashMap<NodeId, MsgHookFn>>,
}

/// Message sent to a network socket.
pub type Payload = Box<dyn Any + Send + Sync>;

type PayloadSender = mpsc::UnboundedSender<Payload>;
type PayloadReceiver = mpsc::UnboundedReceiver<Payload>;
type MsgHookFn = Arc<dyn Fn(&Payload) -> bool + Send + Sync>;

impl plugin::Simulator for NetSim {
    fn new(_rand: &GlobalRng, _time: &TimeHandle, _config: &crate::Config) -> Self {
        unreachable!()
    }

    fn new1(rand: &GlobalRng, time: &TimeHandle, task: &Spawner, config: &crate::Config) -> Self {
        NetSim {
            network: Mutex::new(Network::new(rand.clone(), config.net.clone())),
            dns: Mutex::new(DnsServer::default()),
            rand: rand.clone(),
            time: time.clone(),
            task: task.clone(),
            hooks_req: Default::default(),
            hooks_rsp: Default::default(),
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
    /// Get [`NetSim`] of the current simulator.
    pub fn current() -> Arc<Self> {
        plugin::simulator()
    }

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
    #[deprecated(since = "0.3.0", note = "use `unclog_node` instead")]
    pub fn connect(&self, id: NodeId) {
        self.unclog_node(id);
    }

    /// Unclog the node.
    pub fn unclog_node(&self, id: NodeId) {
        self.network.lock().unclog_node(id, Direction::Both);
    }

    /// Unclog the node for receive.
    pub fn unclog_node_in(&self, id: NodeId) {
        self.network.lock().unclog_node(id, Direction::In);
    }

    /// Unclog the node for send.
    pub fn unclog_node_out(&self, id: NodeId) {
        self.network.lock().unclog_node(id, Direction::Out);
    }

    /// Disconnect a node from the network.
    #[deprecated(since = "0.3.0", note = "use `clog_node` instead")]
    pub fn disconnect(&self, id: NodeId) {
        self.clog_node(id);
    }

    /// Clog the node.
    pub fn clog_node(&self, id: NodeId) {
        self.network.lock().clog_node(id, Direction::Both);
    }

    /// Clog the node for receive.
    pub fn clog_node_in(&self, id: NodeId) {
        self.network.lock().clog_node(id, Direction::In);
    }

    /// Clog the node for send.
    pub fn clog_node_out(&self, id: NodeId) {
        self.network.lock().clog_node(id, Direction::Out);
    }

    /// Connect a pair of nodes.
    #[deprecated(since = "0.3.0", note = "call `unclog_link` twice instead")]
    pub fn connect2(&self, node1: NodeId, node2: NodeId) {
        let mut network = self.network.lock();
        network.unclog_link(node1, node2);
        network.unclog_link(node2, node1);
    }

    /// Unclog the link from `src` to `dst`.
    pub fn unclog_link(&self, src: NodeId, dst: NodeId) {
        self.network.lock().unclog_link(src, dst);
    }

    /// Disconnect a pair of nodes.
    #[deprecated(since = "0.3.0", note = "call `clog_link` twice instead")]
    pub fn disconnect2(&self, node1: NodeId, node2: NodeId) {
        let mut network = self.network.lock();
        network.clog_link(node1, node2);
        network.clog_link(node2, node1);
    }

    /// Clog the link from `src` to `dst`.
    pub fn clog_link(&self, src: NodeId, dst: NodeId) {
        self.network.lock().clog_link(src, dst);
    }

    /// Add a DNS record for the cluster.
    pub fn add_dns_record(&self, hostname: &str, ip: IpAddr) {
        self.dns.lock().add(hostname, ip);
    }

    /// Performs a DNS lookup.
    pub fn lookup_host(&self, hostname: &str) -> Option<IpAddr> {
        self.dns.lock().lookup(hostname)
    }

    /// Add a hook function for RPC requests.
    ///
    /// If the hook function returns `false`, the request will be dropped.
    #[cfg(feature = "rpc")]
    #[cfg_attr(docsrs, doc(cfg(feature = "rpc")))]
    pub fn hook_rpc_req<R: 'static>(
        &self,
        node: NodeId,
        f: impl Fn(&R) -> bool + Send + Sync + 'static,
    ) {
        self.hooks_req.lock().insert(
            node,
            Arc::new(move |payload| {
                if let Some((_, payload)) = payload.downcast_ref::<(u64, Payload)>() {
                    if let Some((_, msg, _)) = payload.downcast_ref::<(u64, R, Bytes)>() {
                        return f(msg);
                    }
                }
                true
            }),
        );
    }

    /// Add a hook function for RPC responses.
    ///
    /// If the hook function returns `false`, the response will be dropped.
    #[cfg(feature = "rpc")]
    #[cfg_attr(docsrs, doc(cfg(feature = "rpc")))]
    pub fn hook_rpc_rsp<R: 'static>(
        &self,
        node: NodeId,
        f: impl Fn(&R) -> bool + Send + Sync + 'static,
    ) {
        self.hooks_rsp.lock().insert(
            node,
            Arc::new(move |payload| {
                if let Some((_, payload)) = payload.downcast_ref::<(u64, Payload)>() {
                    if let Some((msg, _)) = payload.downcast_ref::<(R, Bytes)>() {
                        return f(msg);
                    }
                }
                true
            }),
        );
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
        self.rand_delay().await?;
        if let Some(hook) = self.hooks_req.lock().get(&node).cloned() {
            if !hook(&msg) {
                return Ok(());
            }
        }
        if let Some((ip, dst_node, socket, latency)) =
            self.network.lock().try_send(node, dst, protocol)
        {
            trace!(?latency, "delay");
            let hook = self.hooks_rsp.lock().get(&dst_node).cloned();
            self.time.add_timer(latency, move || {
                if let Some(hook) = hook {
                    if !hook(&msg) {
                        return;
                    }
                }
                socket.deliver((ip, port).into(), dst, msg);
            });
        }
        Ok(())
    }

    /// Opens a new connection to destination.
    // TODO: rename
    pub(crate) async fn connect1(
        self: &Arc<Self>,
        node: NodeId,
        port: u16,
        dst: SocketAddr,
        protocol: IpProtocol,
    ) -> io::Result<(PayloadSender, PayloadReceiver, SocketAddr)> {
        self.rand_delay().await?;
        let (ip, dst_node, socket, latency) = (self.network.lock().try_send(node, dst, protocol))
            .ok_or_else(|| {
            io::Error::new(io::ErrorKind::ConnectionRefused, "connection refused")
        })?;
        let src = (ip, port).into();
        let (tx1, rx1) = self.channel(node, dst, protocol);
        let (tx2, rx2) = self.channel(dst_node, src, protocol);
        trace!(?latency, "delay");
        self.time.add_timer(latency, move || {
            socket.new_connection(src, dst, tx2, rx1);
        });
        Ok((tx1, rx2, src))
    }

    /// Create a reliable, ordered channel between two endpoints.
    fn channel<T: Send + 'static>(
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
                        Some((_, _, _, latency)) => {
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
    node: Arc<NodeInfo>,
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
        let node = crate::context::current_task().node.clone();

        // attempt to bind to each address
        let mut last_err = None;
        for addr in lookup_host(addr).await? {
            net.rand_delay().await?;
            match net
                .network
                .lock()
                .bind(node.id, addr, protocol, socket.clone())
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
        // avoid interfering with restarted node
        if self.node.is_killed() {
            return;
        }
        // avoid panic on panicking
        if let Some(mut network) = self.net.network.try_lock() {
            network.close(self.node.id, self.addr, self.protocol);
        }
    }
}
