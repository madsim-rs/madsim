use crate::{executor::Spawner, rand::RandomHandle, time::TimeHandle};
use bytes::Bytes;
use log::*;
use std::{
    collections::{HashMap, HashSet},
    net::SocketAddr,
    time::Duration,
};

/// A simulated network.
pub(crate) struct Network {
    rand: RandomHandle,
    time: TimeHandle,
    spawner: Spawner,
    config: Config,
    endpoints: HashMap<SocketAddr, async_channel::Sender<Message>>,
    clogged: HashSet<SocketAddr>,
}

#[derive(Debug, Default)]
pub struct Config {
    pub packet_loss_rate: f64,
    pub send_latency: Duration,
}

impl Network {
    pub fn new(rand: RandomHandle, time: TimeHandle, spawner: Spawner, config: Config) -> Self {
        Self {
            rand,
            time,
            spawner,
            config,
            endpoints: HashMap::new(),
            clogged: HashSet::new(),
        }
    }

    pub fn insert(&mut self, target: SocketAddr) -> async_channel::Receiver<Message> {
        assert!(
            !self.endpoints.contains_key(&target),
            "address already exists"
        );
        let (sender, recver) = async_channel::unbounded();
        self.endpoints.insert(target, sender);
        recver
    }

    pub fn remove(&mut self, target: &SocketAddr) {
        self.endpoints.remove(target);
        self.clogged.remove(target);
    }

    pub fn clog(&mut self, target: SocketAddr) {
        self.clogged.insert(target);
    }

    pub fn unclog(&mut self, target: SocketAddr) {
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
        let sender = self.endpoints[&dst].clone();
        let msg = Message {
            tag,
            data: Bytes::copy_from_slice(data),
            from: src,
        };
        trace!("delay: {:?}", self.config.send_latency);
        let delay = self.time.sleep(self.config.send_latency);
        self.spawner.spawn(async move {
            delay.await;
            let _ = sender.try_send(msg);
        })
    }
}

pub struct Message {
    pub tag: u64,
    pub data: Bytes,
    pub from: SocketAddr,
}
