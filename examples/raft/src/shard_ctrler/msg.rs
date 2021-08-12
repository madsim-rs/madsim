use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub type Gid = u64;
pub type ConfigId = u64;

// A configuration -- an assignment of shards to groups.
// Please don't change this.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Config {
    /// config number
    pub num: ConfigId,
    /// shard -> gid
    pub shards: Vec<Gid>,
    /// gid -> servers[]
    pub groups: HashMap<Gid, Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JoinRequest {
    /// new GID -> servers mappings
    pub groups: HashMap<Gid, Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum JoinReply {
    Ok,
    WrongLeader,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaveRequest {
    pub gids: Vec<Gid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LeaveReply {
    Ok,
    WrongLeader,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoveRequest {
    pub shard: u64,
    pub gid: Gid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MoveReply {
    Ok,
    WrongLeader,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryRequest {
    /// desired config number
    pub num: ConfigId,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum QueryReply {
    Ok(Config),
    WrongLeader,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaftCmd {
    pub id: u64,
    pub op: Op,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Op {
    Join(JoinRequest),
    Leave(LeaveRequest),
    Move(MoveRequest),
}
