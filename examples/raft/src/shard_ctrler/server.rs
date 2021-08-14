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

#[derive(Debug, Serialize, Deserialize)]
pub struct ShardInfo {
    /// The index equals the config number, start from 0.
    config: Vec<Config>,
    /// A circular queue with max capacity 50
    ids: Vec<u64>,
}

impl Default for ShardInfo {
    fn default() -> Self {
        ShardInfo {
            config: vec![Config::default()],
            ids: vec![],
        }
    }
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
            Op::Query { num: None } => {
                return Some(self.config.last().unwrap().clone());
            }
            Op::Query { num: Some(num) } => {
                return Some(self.config[num as usize].clone());
            }
            Op::Join { groups } if unique => {
                let mut cfg = self.config.last().unwrap().clone();
                cfg.num += 1;
                cfg.groups.extend(groups);
                self.config.push(cfg);
                // TODO: shard
            }
            Op::Leave { gids } if unique => {
                let mut cfg = self.config.last().unwrap().clone();
                cfg.num += 1;
                for gid in gids {
                    cfg.groups.remove(&gid);
                }
                self.config.push(cfg);
                // TODO: shard
            }
            Op::Move { shard, gid } if unique => {
                let mut cfg = self.config.last().unwrap().clone();
                cfg.num += 1;
                cfg.shards.insert(shard, gid);
                self.config.push(cfg);
            }
            _ => {}
        }
        None
    }
}
