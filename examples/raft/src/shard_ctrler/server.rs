use super::msg::*;
use crate::kvraft::server::{Server, State};
use madsim::{
    fs, net,
    rand::{self, Rng},
    task,
    time::{timeout, Duration},
};
use serde::{Deserialize, Serialize};
use std::{
    net::SocketAddr,
    sync::{Arc, Mutex},
};

pub type ShardCtrler = Server<ShardInfo>;

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct ShardInfo {
    config: Config,
    // A circular queue with max capacity 50
    ids: Vec<u64>,
}

impl State for ShardInfo {
    type Command = Op;
    type Output = Option<Config>;

    fn apply(&mut self, id: u64, cmd: Self::Command) -> Self::Output {
        let unique = !self.ids.contains(&id);
        if self.ids.len() > 50 {
            self.ids.remove(0);
        }
        self.ids.push(id);
        match cmd {
            // TODO: query specific num
            Op::Query { .. } => return Some(self.config.clone()),
            Op::Join { groups } if unique => {
                self.config.groups.extend(groups);
                // TODO: shard
            }
            Op::Leave { gids } if unique => {
                for gid in gids {
                    self.config.groups.remove(&gid);
                }
                // TODO: shard
            }
            Op::Move { shard, gid } if unique => {
                let new_size = self.config.shards.len().max(shard + 1);
                self.config.shards.resize(new_size, 0);
                self.config.shards[shard] = gid;
            }
            _ => {}
        }
        None
    }
}
