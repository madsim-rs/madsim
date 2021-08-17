use super::msg::*;
use crate::kvraft::client::ClerkCore;
use madsim::rand::{self, Rng};
use std::{collections::HashMap, net::SocketAddr};

pub struct Clerk {
    core: ClerkCore,
}

impl Clerk {
    pub fn new(servers: Vec<SocketAddr>) -> Clerk {
        Clerk {
            core: ClerkCore::new(servers),
        }
    }

    pub async fn query(&self, num: Option<u64>) -> Config {
        let id: u64 = rand::rng().gen();
        let args = Op::Query { num };
        self.core
            .call::<_, Option<Config>>((id, args))
            .await
            .unwrap()
    }

    pub async fn join(&self, groups: HashMap<Gid, Vec<SocketAddr>>) {
        let id: u64 = rand::rng().gen();
        let args = Op::Join { groups };
        self.core.call::<_, Option<Config>>((id, args)).await;
    }

    pub async fn leave(&self, gids: Vec<u64>) {
        let id: u64 = rand::rng().gen();
        let args = Op::Leave { gids };
        self.core.call::<_, Option<Config>>((id, args)).await;
    }

    pub async fn move_(&self, shard: usize, gid: u64) {
        let id: u64 = rand::rng().gen();
        let args = Op::Move { shard, gid };
        self.core.call::<_, Option<Config>>((id, args)).await;
    }
}
