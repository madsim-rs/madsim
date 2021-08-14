use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PutAppendArgs {
    pub id: u64,
    pub key: String,
    pub value: String,
    pub append: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PutAppendReply;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetArgs {
    pub key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetReply {
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaftCmd {
    pub id: u64,
    pub op: Op,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Op {
    Get { key: String },
    Put { key: String, value: String },
    Append { key: String, value: String },
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
