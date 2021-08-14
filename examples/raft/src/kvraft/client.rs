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
    servers: Vec<SocketAddr>,
    // You will have to modify this struct.
    leader: AtomicUsize,
}

impl Clerk {
    pub fn new(servers: Vec<SocketAddr>) -> Clerk {
        // You'll have to add code here.
        Clerk {
            servers,
            leader: AtomicUsize::new(0),
        }
    }

    /// fetch the current value for a key.
    /// returns "" if the key does not exist.
    /// keeps trying forever in the face of all other errors.
    pub async fn get(&self, key: String) -> String {
        let args = GetArgs { key };
        self.call::<_, GetReply>(args).await.value
    }

    pub async fn put(&self, key: String, value: String) {
        let args = PutAppendArgs {
            key,
            value,
            append: false,
            id: rand::rng().gen(),
        };
        self.call::<_, PutAppendReply>(args).await;
    }

    pub async fn append(&self, key: String, value: String) {
        let args = PutAppendArgs {
            key,
            value,
            append: true,
            id: rand::rng().gen(),
        };
        self.call::<_, PutAppendReply>(args).await;
    }

    async fn call<Req, Rsp>(&self, args: Req) -> Rsp
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
