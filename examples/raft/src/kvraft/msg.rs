use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PutAppendArgs {
    pub id: u64,
    pub key: String,
    pub value: String,
    pub append: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PutAppendReply {
    Ok,
    WrongLeader,
    Failed,
    Timeout,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetArgs {
    pub key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GetReply {
    Ok { value: String },
    WrongLeader,
    Failed,
    Timeout,
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
