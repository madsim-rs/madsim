use super::msg::*;
use madsim::{
    net,
    rand::{self, Rng},
    time::*,
};
use std::{
    net::SocketAddr,
    sync::atomic::{AtomicUsize, Ordering},
};

pub struct Clerk {
    core: ClerkCore,
}

impl Clerk {
    pub fn new(servers: Vec<SocketAddr>) -> Clerk {
        Clerk {
            core: ClerkCore::new(servers),
        }
    }

    /// fetch the current value for a key.
    /// returns "" if the key does not exist.
    /// keeps trying forever in the face of all other errors.
    pub async fn get(&self, key: String) -> String {
        let id: u64 = rand::rng().gen();
        let args = Op::Get { key };
        self.core.call::<_, String>((id, args)).await
    }

    pub async fn put(&self, key: String, value: String) {
        let id: u64 = rand::rng().gen();
        let args = Op::Put { key, value };
        self.core.call::<_, String>((id, args)).await;
    }

    pub async fn append(&self, key: String, value: String) {
        let id: u64 = rand::rng().gen();
        let args = Op::Append { key, value };
        self.core.call::<_, String>((id, args)).await;
    }
}

pub struct ClerkCore {
    servers: Vec<SocketAddr>,
    leader: AtomicUsize,
}

impl ClerkCore {
    pub fn new(servers: Vec<SocketAddr>) -> ClerkCore {
        ClerkCore {
            servers,
            leader: AtomicUsize::new(0),
        }
    }

    pub async fn call<Req, Rsp>(&self, args: Req) -> Rsp
    where
        Req: net::Message + Clone,
        Rsp: net::Message,
    {
        let net = net::NetworkLocalHandle::current();
        let mut i = self.leader.load(Ordering::Relaxed);
        loop {
            debug!("->{} {:?}", i, args);
            match net
                .call_timeout::<Req, Result<Rsp, Error>>(
                    self.servers[i],
                    args.clone(),
                    Duration::from_millis(500),
                )
                .await
            {
                // client side error
                Err(e) => {
                    debug!("<-{} {:?}", i, e);
                    i = (i + 1) % self.servers.len();
                    continue;
                }
                // server side error
                Ok(Err(e)) => {
                    debug!("<-{} {:?}", i, e);
                    if let Error::NotLeader(leader) = e {
                        i = leader;
                    } else {
                        i = (i + 1) % self.servers.len();
                    }
                    continue;
                }
                Ok(Ok(v)) => {
                    debug!("<-{} ok", i);
                    self.leader.store(i, Ordering::Relaxed);
                    return v;
                }
            }
        }
    }
}
