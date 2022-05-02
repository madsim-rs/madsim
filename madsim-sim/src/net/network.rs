use crate::{rand::*, time::TimeHandle, NodeId};
use futures::channel::oneshot;
use log::*;
use serde::{Deserialize, Serialize};
use std::{
    any::Any,
    collections::{hash_map::Entry, HashMap, HashSet},
    hash::{Hash, Hasher},
    io,
    net::SocketAddr,
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
    addr_to_node: HashMap<SocketAddr, NodeId>,
    clogged_node: HashSet<NodeId>,
    clogged_link: HashSet<(NodeId, NodeId)>,
}

/// Network for a node.
#[derive(Default)]
struct Node {
    sockets: HashMap<SocketAddr, Arc<Mutex<Mailbox>>>,
}

/// Network configurations.
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
        assert!(self.nodes.contains_key(&id));
        debug!("reset: {id}");
        if let Some(node) = self.nodes.insert(id, Default::default()) {
            for addr in node.sockets.keys() {
                self.addr_to_node.remove(addr);
            }
        }
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

    pub fn bind(&mut self, node: NodeId, addr: SocketAddr) -> io::Result<()> {
        debug!("bind: {addr} -> {node}");
        match self.addr_to_node.entry(addr) {
            Entry::Occupied(_) => Err(io::Error::new(
                io::ErrorKind::AddrInUse,
                "address already in use",
            )),
            Entry::Vacant(o) => {
                o.insert(node);
                self.nodes
                    .get_mut(&node)
                    .unwrap()
                    .sockets
                    .insert(addr, Default::default());
                Ok(())
            }
        }
    }

    pub fn close(&mut self, addr: SocketAddr) {
        debug!("close: {addr}");
        if let Some(node) = self.addr_to_node.remove(&addr) {
            if let Some(node) = self.nodes.get_mut(&node) {
                node.sockets.remove(&addr);
            }
        }
    }

    pub fn send(&mut self, src: SocketAddr, dst: SocketAddr, tag: u64, data: Payload) {
        trace!("send: {src} -> {dst}, tag={tag}");
        let src_node = self.addr_to_node[&src];
        let dst_node = match self.addr_to_node.get(&dst) {
            Some(x) => *x,
            None => {
                trace!("destination not found: {dst}");
                return;
            }
        };
        if self.clogged_node.contains(&src_node)
            || self.clogged_node.contains(&dst_node)
            || self.clogged_link.contains(&(src_node, dst_node))
        {
            trace!("clogged");
            return;
        }
        if self.rand.gen_bool(self.config.packet_loss_rate) {
            trace!("packet loss");
            return;
        }
        let ep = self.nodes[&dst_node].sockets[&dst].clone();
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

    pub fn recv(&mut self, dst: SocketAddr, tag: u64) -> oneshot::Receiver<Message> {
        let node = self.addr_to_node.get(&dst).expect("socket not found");
        self.nodes[node].sockets[&dst].lock().unwrap().recv(tag)
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
