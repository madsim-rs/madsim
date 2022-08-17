use super::{Payload, PayloadReceiver, PayloadSender};
use crate::{
    rand::*,
    task::{JoinHandle, NodeId},
};
use async_task::FallibleTask;
use log::*;
use serde::{Deserialize, Serialize};
use std::{
    any::Any,
    collections::{hash_map::Entry, HashMap, HashSet},
    hash::{Hash, Hasher},
    io,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    ops::Range,
    sync::Arc,
    time::Duration,
};

/// A simulated network.
///
/// This object manages the links and address resolution.
/// It doesn't care about specific communication protocol.
pub(crate) struct Network {
    rand: GlobalRng,
    config: Config,
    stat: Stat,
    nodes: HashMap<NodeId, Node>,
    /// Maps the global IP to its node.
    addr_to_node: HashMap<IpAddr, NodeId>,
    clogged_node: HashSet<NodeId>,
    clogged_link: HashSet<(NodeId, NodeId)>,
}

/// A node in the network.
#[derive(Default)]
struct Node {
    /// IP address of the node.
    ///
    /// NOTE: now a node can have at most one IP address.
    ip: Option<IpAddr>,
    /// Sockets in the node.
    sockets: HashMap<(SocketAddr, IpProtocol), Arc<dyn Socket>>,
    /// Used to close channels when the node is reset.
    tasks: Vec<FallibleTask<()>>,
}

#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IpProtocol {
    Tcp,
    Udp,
}

/// Upper-level protocol should implement its own socket type.
pub trait Socket: Any + Send + Sync {
    /// Deliver a message from other socket.
    fn deliver(&self, _src: SocketAddr, _dst: SocketAddr, _msg: Payload) {}

    /// A new connection request.
    fn new_connection(
        &self,
        _src: SocketAddr,
        _dst: SocketAddr,
        _tx: PayloadSender,
        _rx: PayloadReceiver,
    ) {
    }
}

/// Network configurations.
#[cfg_attr(docsrs, doc(cfg(madsim)))]
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct Config {
    /// Possibility of packet loss.
    #[serde(default)]
    pub packet_loss_rate: f64,
    /// The latency range of sending packets.
    #[serde(default = "default_send_latency")]
    pub send_latency: Range<Duration>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            packet_loss_rate: 0.0,
            send_latency: default_send_latency(),
        }
    }
}

const fn default_send_latency() -> Range<Duration> {
    Duration::from_millis(1)..Duration::from_millis(10)
}

#[allow(clippy::derive_hash_xor_eq)]
impl Hash for Config {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.packet_loss_rate.to_bits().hash(state);
        self.send_latency.hash(state);
    }
}

/// Network statistics.
#[cfg_attr(docsrs, doc(cfg(madsim)))]
#[derive(Debug, Default, Clone)]
pub struct Stat {
    /// Total number of messages.
    pub msg_count: u64,
}

impl Network {
    pub fn new(rand: GlobalRng, config: Config) -> Self {
        Self {
            rand,
            config,
            stat: Stat::default(),
            nodes: HashMap::new(),
            addr_to_node: HashMap::new(),
            clogged_node: HashSet::new(),
            clogged_link: HashSet::new(),
        }
    }

    pub fn update_config(&mut self, f: impl FnOnce(&mut Config)) {
        f(&mut self.config);
    }

    pub fn stat(&self) -> &Stat {
        &self.stat
    }

    pub fn insert_node(&mut self, id: NodeId) {
        debug!("insert: {id}");
        self.nodes.insert(id, Default::default());
    }

    pub fn reset_node(&mut self, id: NodeId) {
        debug!("reset: {id}");
        let node = self.nodes.get_mut(&id).expect("node not found");
        // close all sockets
        node.sockets.clear();
        node.tasks.clear();
    }

    pub fn set_ip(&mut self, id: NodeId, ip: IpAddr) {
        debug!("set-ip: {id}: {ip}");
        let node = self.nodes.get_mut(&id).expect("node not found");
        if let Some(old_ip) = node.ip.replace(ip) {
            self.addr_to_node.remove(&old_ip);
        }
        let old_node = self.addr_to_node.insert(ip, id);
        if let Some(old_node) = old_node {
            panic!("IP conflict: {ip} {old_node}");
        }
        // TODO: what if we change the IP when there are opening sockets?
    }

    pub fn clog_node(&mut self, id: NodeId) {
        assert!(self.nodes.contains_key(&id));
        debug!("clog: {id}");
        self.clogged_node.insert(id);
    }

    pub fn unclog_node(&mut self, id: NodeId) {
        assert!(self.nodes.contains_key(&id));
        debug!("unclog: {id}");
        self.clogged_node.remove(&id);
    }

    pub fn clog_link(&mut self, src: NodeId, dst: NodeId) {
        assert!(self.nodes.contains_key(&src));
        assert!(self.nodes.contains_key(&dst));
        debug!("clog: {src} -> {dst}");
        self.clogged_link.insert((src, dst));
    }

    pub fn unclog_link(&mut self, src: NodeId, dst: NodeId) {
        assert!(self.nodes.contains_key(&src));
        assert!(self.nodes.contains_key(&dst));
        debug!("unclog: {src} -> {dst}");
        self.clogged_link.remove(&(src, dst));
    }

    /// Returns whether a link is clogged.
    pub fn link_clogged(&self, src: NodeId, dst: NodeId) -> bool {
        self.clogged_node.contains(&src)
            || self.clogged_node.contains(&dst)
            || self.clogged_link.contains(&(src, dst))
    }

    /// Bind a socket to the specified address.
    pub fn bind(
        &mut self,
        node_id: NodeId,
        mut addr: SocketAddr,
        protocol: IpProtocol,
        socket: Arc<dyn Socket>,
    ) -> io::Result<SocketAddr> {
        let origin_addr = addr;
        let node = self.nodes.get_mut(&node_id).expect("node not found");
        // check IP address
        if !addr.ip().is_unspecified()
            && !addr.ip().is_loopback()
            && node.ip.is_some()
            && addr.ip() != node.ip.unwrap()
        {
            return Err(io::Error::new(
                io::ErrorKind::AddrNotAvailable,
                format!("invalid address: {addr}"),
            ));
        }
        // resolve port if unspecified
        if addr.port() == 0 {
            let port = (1..=u16::MAX)
                .find(|&port| {
                    !node
                        .sockets
                        .contains_key(&((addr.ip(), port).into(), protocol))
                })
                .ok_or_else(|| {
                    io::Error::new(io::ErrorKind::AddrInUse, "no available ephemeral port")
                })?;
            addr.set_port(port);
        }
        // insert socket
        match node.sockets.entry((addr, protocol)) {
            Entry::Occupied(_) => {
                return Err(io::Error::new(
                    io::ErrorKind::AddrInUse,
                    format!("address already in use: {addr}"),
                ))
            }
            Entry::Vacant(o) => {
                o.insert(socket);
            }
        }
        debug!("bind: {node_id} {origin_addr} -> {addr}");
        Ok(addr)
    }

    /// Close a socket.
    pub fn close(&mut self, node: NodeId, addr: SocketAddr, protocol: IpProtocol) {
        debug!("close: {node} {addr} {protocol:?}");
        let node = self.nodes.get_mut(&node).expect("node not found");
        node.sockets.remove(&(addr, protocol));
    }

    /// Returns the latency of sending a packet. If packet loss, returns `None`.
    fn test_link(&mut self, src: NodeId, dst: NodeId) -> Option<Duration> {
        if self.link_clogged(src, dst) || self.rand.gen_bool(self.config.packet_loss_rate) {
            None
        } else {
            self.stat.msg_count += 1;
            // TODO: special value for loopback
            Some(self.rand.gen_range(self.config.send_latency.clone()))
        }
    }

    /// Resolve destination node from IP address.
    pub fn resolve_dest_node(
        &self,
        node: NodeId,
        dst: SocketAddr,
        protocol: IpProtocol,
    ) -> Option<NodeId> {
        let node0 = self.nodes.get(&node).expect("node not found");
        if dst.ip().is_loopback() || node0.sockets.contains_key(&(dst, protocol)) {
            Some(node)
        } else if node0.ip.is_none() {
            warn!("ip not set: {node}");
            None
        } else if let Some(x) = self.addr_to_node.get(&dst.ip()) {
            Some(*x)
        } else {
            warn!("destination not found: {dst}");
            None
        }
    }

    /// Try sending a message to the destination.
    ///
    /// If destination is not found or packet loss, returns `None`.
    /// Otherwise returns the source IP, socket and latency.
    pub fn try_send(
        &mut self,
        node: NodeId,
        dst: SocketAddr,
        protocol: IpProtocol,
    ) -> Option<(IpAddr, NodeId, Arc<dyn Socket>, Duration)> {
        let dst_node = self.resolve_dest_node(node, dst, protocol)?;
        let latency = self.test_link(node, dst_node)?;
        let sockets = &self.nodes.get(&dst_node)?.sockets;
        let ep = (sockets.get(&(dst, protocol)))
            .or_else(|| sockets.get(&((Ipv4Addr::UNSPECIFIED, dst.port()).into(), protocol)))?;
        let src_ip = if dst.ip().is_loopback() {
            IpAddr::V4(Ipv4Addr::LOCALHOST)
        } else {
            self.nodes.get(&node).expect("node not found").ip.unwrap()
        };
        Some((src_ip, dst_node, ep.clone(), latency))
    }

    pub fn abort_task_on_reset(&mut self, node: NodeId, handle: JoinHandle<()>) {
        let node = self.nodes.get_mut(&node).expect("node not found");
        node.tasks.push(handle.cancel_on_drop());
    }
}
