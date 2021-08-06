use std::net::SocketAddr;

pub struct RaftNode {}

#[derive(Clone)]
pub struct RaftHandle {}

impl RaftHandle {
    pub fn new(peers: Vec<SocketAddr>, me: usize) -> Self {
        RaftHandle {}
    }

    /// The current term of this peer.
    pub fn term(&self) -> u64 {
        todo!()
    }

    /// Whether this peer believes it is the leader.
    pub fn is_leader(&self) -> bool {
        todo!()
    }
}
