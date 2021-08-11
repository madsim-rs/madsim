use crate::{rand::*, time::TimeHandle};
use futures::channel::oneshot;
use log::*;
use std::{
    collections::{HashMap, HashSet},
    net::SocketAddr,
    ops::Range,
    sync::{Arc, Mutex},
    time::Duration,
};

/// A simulated network.
pub(crate) struct Network {
    rand: RandomHandle,
    time: TimeHandle,
    config: Config,
    stat: Stat,
    endpoints: HashMap<SocketAddr, Arc<Mutex<Endpoint>>>,
    clogged: HashSet<SocketAddr>,
    clogged_link: HashSet<(SocketAddr, SocketAddr)>,
}

#[derive(Debug)]
pub struct Config {
    pub packet_loss_rate: f64,
    pub send_latency: Range<Duration>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            packet_loss_rate: 0.0,
            send_latency: Duration::from_millis(1)..Duration::from_millis(10),
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct Stat {
    pub msg_count: u64,
}

impl Network {
    pub fn new(rand: RandomHandle, time: TimeHandle) -> Self {
        Self {
            rand,
            time,
            config: Config::default(),
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
        if self.endpoints.contains_key(&target) {
            return;
        }
        trace!("insert: {}", target);
        self.endpoints.insert(target, Default::default());
    }

    pub fn remove(&mut self, target: &SocketAddr) {
        trace!("remove: {}", target);
        self.endpoints.remove(target);
        self.clogged.remove(target);
    }

    pub fn clog(&mut self, target: SocketAddr) {
        assert!(self.endpoints.contains_key(&target));
        trace!("clog: {}", target);
        self.clogged.insert(target);
    }

    pub fn unclog(&mut self, target: SocketAddr) {
        assert!(self.endpoints.contains_key(&target));
        trace!("unclog: {}", target);
        self.clogged.remove(&target);
    }

    pub fn clog_link(&mut self, src: SocketAddr, dst: SocketAddr) {
        assert!(self.endpoints.contains_key(&src));
        assert!(self.endpoints.contains_key(&dst));
        trace!("clog: {} -> {}", src, dst);
        self.clogged_link.insert((src, dst));
    }

    pub fn unclog_link(&mut self, src: SocketAddr, dst: SocketAddr) {
        assert!(self.endpoints.contains_key(&src));
        assert!(self.endpoints.contains_key(&dst));
        trace!("unclog: {} -> {}", src, dst);
        self.clogged_link.remove(&(src, dst));
    }

    pub fn send(&mut self, src: SocketAddr, dst: SocketAddr, tag: u64, data: &[u8]) {
        trace!("send: {} -> {}, tag={}, len={}", src, dst, tag, data.len());
        assert!(self.endpoints.contains_key(&src));
        if !self.endpoints.contains_key(&dst)
            || self.clogged.contains(&src)
            || self.clogged.contains(&dst)
            || self.clogged_link.contains(&(src, dst))
            || self.rand.should_fault(self.config.packet_loss_rate)
        {
            trace!("drop");
            return;
        }
        let ep = self.endpoints[&dst].clone();
        let msg = Message {
            tag,
            data: data.into(),
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
    pub data: Vec<u8>,
    pub from: SocketAddr,
}

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
