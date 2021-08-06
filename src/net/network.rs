use crate::{rand::*, time::TimeHandle};
use async_channel::{Receiver, Sender};
use bytes::Bytes;
use log::*;
use std::{
    collections::{HashMap, HashSet},
    net::SocketAddr,
    ops::Range,
    time::Duration,
};

/// A simulated network.
pub(crate) struct Network {
    rand: RandomHandle,
    time: TimeHandle,
    config: Config,
    stat: Stat,
    endpoints: HashMap<SocketAddr, (Sender<Message>, Receiver<Message>)>,
    clogged: HashSet<SocketAddr>,
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
        }
    }

    pub fn update_config(&mut self, f: impl FnOnce(&mut Config)) {
        f(&mut self.config);
    }

    pub fn stat(&self) -> &Stat {
        &self.stat
    }

    pub fn get(&mut self, target: SocketAddr) -> Receiver<Message> {
        if let Some((_, recver)) = self.endpoints.get(&target) {
            return recver.clone();
        }
        trace!("insert: {}", target);
        let (sender, recver) = async_channel::unbounded();
        self.endpoints.insert(target, (sender, recver.clone()));
        recver
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

    pub fn send(&mut self, src: SocketAddr, dst: SocketAddr, tag: u64, data: &[u8]) {
        trace!("send: {} -> {}, tag={}, len={}", src, dst, tag, data.len());
        assert!(self.endpoints.contains_key(&src));
        if !self.endpoints.contains_key(&dst)
            || self.clogged.contains(&src)
            || self.clogged.contains(&dst)
            || self.rand.should_fault(self.config.packet_loss_rate)
        {
            trace!("drop");
            return;
        }
        let sender = self.endpoints[&dst].0.clone();
        let msg = Message {
            tag,
            data: Bytes::copy_from_slice(data),
            from: src,
        };
        let latency = self.rand.gen_range(self.config.send_latency.clone());
        trace!("delay: {:?}", latency);
        self.time.add_timer(self.time.now() + latency, move || {
            let _ = sender.try_send(msg);
        });
        self.stat.msg_count += 1;
    }
}

pub struct Message {
    pub tag: u64,
    pub data: Bytes,
    pub from: SocketAddr,
}
