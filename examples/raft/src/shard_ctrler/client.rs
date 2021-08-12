use super::msg::*;
use madsim::{
    net,
    rand::{self, Rng},
    time::*,
};
use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::atomic::{AtomicUsize, Ordering},
};

pub struct Clerk {
    servers: Vec<SocketAddr>,
    // You will have to modify this struct.
    leader: AtomicUsize,
}

impl Clerk {
    pub fn new(servers: Vec<SocketAddr>) -> Clerk {
        // You'll have to add code here.
        Clerk {
            servers,
            leader: AtomicUsize::new(0),
        }
    }

    pub async fn query(&self, num: Option<u64>) -> Config {
        todo!()
    }

    pub async fn join(&self, groups: HashMap<Gid, Vec<String>>) {
        todo!()
    }

    pub async fn leave(&self, gids: Vec<u64>) {
        todo!()
    }

    pub async fn move_(&self, shard: u64, gid: u64) {
        todo!()
    }
}
