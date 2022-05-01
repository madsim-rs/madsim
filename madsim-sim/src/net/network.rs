use crate::{rand::*, time::TimeHandle};
use futures::channel::oneshot;
use log::*;
use serde::{Deserialize, Serialize};
use std::{
    any::Any,
    collections::{HashMap, HashSet},
    hash::{Hash, Hasher},
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
    endpoints: HashMap<SocketAddr, Arc<Mutex<Endpoint>>>,
    clogged: HashSet<SocketAddr>,
    clogged_link: HashSet<(SocketAddr, SocketAddr)>,
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
            endpoints: HashMap::new(),
            clogged: HashSet::new(),
            clogged_link: HashSet::new(),
        }
    }

    pub fn update_config(&mut self, f: impl FnOnce(&mut Config)) {
        f(&mut self.config);
    }

    pub fn stat(&self) -> &Stat {
        &self.stat
    }

    pub fn insert(&mut self, target: SocketAddr) {
        debug!("insert: {}", target);
        self.endpoints.insert(target, Default::default());
    }

    #[allow(dead_code)]
    pub fn remove(&mut self, target: &SocketAddr) {
        debug!("remove: {}", target);
        self.endpoints.remove(target);
        self.clogged.remove(target);
    }

    pub fn reset(&mut self, target: SocketAddr) {
        assert!(self.endpoints.contains_key(&target));
        debug!("reset: {}", target);
        self.endpoints.insert(target, Default::default());
    }

    pub fn clog(&mut self, target: SocketAddr) {
        assert!(self.endpoints.contains_key(&target));
        debug!("clog: {}", target);
        self.clogged.insert(target);
    }

    pub fn unclog(&mut self, target: SocketAddr) {
        assert!(self.endpoints.contains_key(&target));
        debug!("unclog: {}", target);
        self.clogged.remove(&target);
    }

    pub fn clog_link(&mut self, src: SocketAddr, dst: SocketAddr) {
        assert!(self.endpoints.contains_key(&src));
        assert!(self.endpoints.contains_key(&dst));
        debug!("clog: {} -> {}", src, dst);
        self.clogged_link.insert((src, dst));
    }

    pub fn unclog_link(&mut self, src: SocketAddr, dst: SocketAddr) {
        assert!(self.endpoints.contains_key(&src));
        assert!(self.endpoints.contains_key(&dst));
        debug!("unclog: {} -> {}", src, dst);
        self.clogged_link.remove(&(src, dst));
    }

    pub fn send(&mut self, src: SocketAddr, dst: SocketAddr, tag: u64, data: Payload) {
        trace!("send: {} -> {}, tag={}", src, dst, tag);
        assert!(self.endpoints.contains_key(&src));
        if !self.endpoints.contains_key(&dst)
            || self.clogged.contains(&src)
            || self.clogged.contains(&dst)
            || self.clogged_link.contains(&(src, dst))
        {
            trace!("no connection");
            return;
        }
        if self.rand.gen_bool(self.config.packet_loss_rate) {
            trace!("packet loss");
            return;
        }
        let ep = self.endpoints[&dst].clone();
        let msg = Message {
            tag,
            data,
            from: src,
        };
        let latency = self.rand.gen_range(self.config.send_latency.clone());
        trace!("delay: {:?}", latency);
        self.time.add_timer(self.time.now() + latency, move || {
            ep.lock().unwrap().send(msg);
        });
        self.stat.msg_count += 1;
    }

    pub fn recv(&mut self, dst: SocketAddr, tag: u64) -> oneshot::Receiver<Message> {
        self.endpoints[&dst].lock().unwrap().recv(tag)
    }
}

pub struct Message {
    pub tag: u64,
    pub data: Payload,
    pub from: SocketAddr,
}

pub type Payload = Box<dyn Any + Send + Sync>;

#[derive(Default)]
struct Endpoint {
    registered: Vec<(u64, oneshot::Sender<Message>)>,
    msgs: Vec<Message>,
}

impl Endpoint {
    fn send(&mut self, msg: Message) {
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
