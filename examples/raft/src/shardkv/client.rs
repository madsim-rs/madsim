use crate::kvraft::client::ClerkCore;
use crate::shard_ctrler::N_SHARDS;
use madsim::rand::{self, Rng};
use std::{collections::HashMap, net::SocketAddr};

// which shard is a key in?
// please use this function,
// and please do not change it.
pub fn key2shard(key: &str) -> usize {
    if let Some(c) = key.bytes().next() {
        c as usize % N_SHARDS
    } else {
        0
    }
}

pub struct Clerk {
    core: ClerkCore,
}

impl Clerk {
    pub fn new(servers: Vec<SocketAddr>) -> Clerk {
        Clerk {
            core: ClerkCore::new(servers),
        }
    }

    pub async fn get(&self, key: String) -> String {
        todo!()
    }

    pub async fn put(&self, key: String, value: String) {
        todo!()
    }

    pub async fn append(&self, key: String, value: String) {
        todo!()
    }
}
