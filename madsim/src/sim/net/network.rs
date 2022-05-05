use crate::{rand::*, task::NodeId, time::TimeHandle};
use futures::channel::oneshot;
use log::*;
use serde::{Deserialize, Serialize};
use std::{
    any::Any,
    collections::{hash_map::Entry, HashMap, HashSet},
    hash::{Hash, Hasher},
    io,
    net::{IpAddr, SocketAddr},
    ops::Range,
    sync::{Arc, Mutex},
    time::Duration,
};

/// A simulated network.
pub(crate) struct Network {
    rand: RandHandle,
    time: TimeHandle,
    config: Config,
    stat: Stat,
    nodes: HashMap<NodeId, Node>,
    /// Maps the global IP to its node.
    addr_to_node: HashMap<IpAddr, NodeId>,
    clogged_node: HashSet<NodeId>,
    clogged_link: HashSet<(NodeId, NodeId)>,
}

/// Network for a node.
#[derive(Default)]
struct Node {
    /// IP address of the node.
    ///
    /// NOTE: now a node can have at most one IP address.
    ip: Option<IpAddr>,
    /// Sockets in the node.
    sockets: HashMap<u16, Arc<Mutex<Mailbox>>>,
}

/// Network configurations.
#[cfg_attr(docsrs, doc(cfg(feature = "sim")))]
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
#[cfg_attr(docsrs, doc(cfg(feature = "sim")))]
#[derive(Debug, Default, Clone)]
pub struct Stat {
    /// Total number of messages.
    pub msg_count: u64,
}

impl Network {
    pub fn new(rand: RandHandle, time: TimeHandle, config: Config) -> Self {
        Self {
            rand,
            time,
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

    pub fn bind(&mut self, node: NodeId, mut addr: SocketAddr) -> io::Result<SocketAddr> {
        debug!("bind: {addr} -> {node}");
        let node = self.nodes.get_mut(&node).expect("node not found");
        // resolve IP if unspecified
        if addr.ip().is_unspecified() {
            if let Some(ip) = node.ip {
                addr.set_ip(ip);
            } else {
                todo!("try to bind 0.0.0.0, but the node IP is also unspecified");
            }
        } else if addr.ip().is_loopback() {
        } else if addr.ip() != node.ip.expect("node IP is unset") {
            return Err(io::Error::new(
                io::ErrorKind::AddrNotAvailable,
                format!("invalid address: {addr}"),
            ));
        }
        // resolve port if unspecified
        if addr.port() == 0 {
            let port = (1..=u16::MAX)
                .find(|port| !node.sockets.contains_key(port))
                .ok_or_else(|| {
                    io::Error::new(io::ErrorKind::AddrInUse, "no available ephemeral port")
                })?;
            addr.set_port(port);
        }
        // insert socket
        match node.sockets.entry(addr.port()) {
            Entry::Occupied(_) => {
                return Err(io::Error::new(
                    io::ErrorKind::AddrInUse,
                    format!("address already in use: {addr}"),
                ))
            }
            Entry::Vacant(o) => {
                o.insert(Default::default());
            }
        }
        Ok(addr)
    }

    pub fn close(&mut self, node: NodeId, addr: SocketAddr) {
        debug!("close: {node} {addr}");
        let node = self.nodes.get_mut(&node).expect("node not found");
        node.sockets.remove(&addr.port());
    }

    pub fn send(
        &mut self,
        node: NodeId,
        src: SocketAddr,
        dst: SocketAddr,
        tag: u64,
        data: Payload,
    ) {
        trace!("send: {node} {src} -> {dst}, tag={tag}");
        let dst_node = match self.addr_to_node.get(&dst.ip()) {
            Some(x) => *x,
            None => {
                trace!("destination not found: {dst}");
                return;
            }
        };
        if self.clogged_node.contains(&node)
            || self.clogged_node.contains(&dst_node)
            || self.clogged_link.contains(&(node, dst_node))
        {
            trace!("clogged");
            return;
        }
        if self.rand.gen_bool(self.config.packet_loss_rate) {
            trace!("packet loss");
            return;
        }
        let ep = self.nodes[&dst_node].sockets[&dst.port()].clone();
        let msg = Message {
            tag,
            data,
            from: src,
        };
        let latency = self.rand.gen_range(self.config.send_latency.clone());
        trace!("delay: {latency:?}");
        self.time.add_timer(self.time.now() + latency, move || {
            ep.lock().unwrap().deliver(msg);
        });
        self.stat.msg_count += 1;
    }

    pub fn recv(&mut self, node: NodeId, dst: SocketAddr, tag: u64) -> oneshot::Receiver<Message> {
        self.nodes[&node].sockets[&dst.port()]
            .lock()
            .unwrap()
            .recv(tag)
    }
}

pub struct Message {
    pub tag: u64,
    pub data: Payload,
    pub from: SocketAddr,
}

pub type Payload = Box<dyn Any + Send + Sync>;

/// Tag message mailbox for an endpoint.
#[derive(Default)]
struct Mailbox {
    /// Pending receive requests.
    registered: Vec<(u64, oneshot::Sender<Message>)>,
    /// Messages that have not been received.
    msgs: Vec<Message>,
}

impl Mailbox {
    fn deliver(&mut self, msg: Message) {
        let mut i = 0;
        let mut msg = Some(msg);
        while i < self.registered.len() {
            if matches!(&msg, Some(msg) if msg.tag == self.registered[i].0) {
                // tag match, take and try send
                let (_, sender) = self.registered.swap_remove(i);
                msg = match sender.send(msg.take().unwrap()) {
                    Ok(_) => return,
                    Err(m) => Some(m),
                };
                // failed to send, try next
            } else {
                // tag mismatch, move to next
                i += 1;
            }
        }
        // failed to match awaiting recv, save
        self.msgs.push(msg.unwrap());
    }

    fn recv(&mut self, tag: u64) -> oneshot::Receiver<Message> {
        let (tx, rx) = oneshot::channel();
        if let Some(idx) = self.msgs.iter().position(|msg| tag == msg.tag) {
            let msg = self.msgs.swap_remove(idx);
            tx.send(msg).ok().unwrap();
        } else {
            self.registered.push((tag, tx));
        }
        rx
    }
}
