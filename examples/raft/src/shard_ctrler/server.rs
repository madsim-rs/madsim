use super::msg::*;
use crate::raft;
use futures::{channel::oneshot, StreamExt};
use madsim::{
    fs, net,
    rand::{self, Rng},
    task,
    time::{timeout, Duration},
};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fmt,
    net::SocketAddr,
    sync::{Arc, Mutex},
};

pub struct ShardCtrler {
    rf: raft::RaftHandle,
    me: usize,
}

impl fmt::Debug for ShardCtrler {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ShardCtrler({})", self.me)
    }
}

impl ShardCtrler {
    pub async fn new(servers: Vec<SocketAddr>, me: usize) -> Arc<ShardCtrler> {
        todo!()
    }

    /// Whether this peer believes it is the leader.
    pub fn is_leader(&self) -> bool {
        self.rf.is_leader()
    }
}
