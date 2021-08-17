use serde::{Deserialize, Serialize};
use std::{collections::HashMap, net::SocketAddr};

pub type Gid = u64;
pub type ConfigId = u64;

// A configuration -- an assignment of shards to groups.
// Please don't change this.
#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Config {
    /// config number
    pub num: ConfigId,
    /// shard -> gid
    pub shards: HashMap<usize, Gid>,
    /// gid -> servers[]
    pub groups: HashMap<Gid, Vec<SocketAddr>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Op {
    Query {
        /// desired config number
        num: Option<ConfigId>,
    },
    Join {
        /// new GID -> servers mappings
        groups: HashMap<Gid, Vec<SocketAddr>>,
    },
    Leave {
        gids: Vec<Gid>,
    },
    Move {
        shard: usize,
        gid: Gid,
    },
}

#[derive(thiserror::Error, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Error {
    #[error("not leader")]
    NotLeader(usize),
    #[error("timeout")]
    Timeout,
    #[error("failed to reach consensus")]
    Failed,
}
