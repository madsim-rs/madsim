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
        // You will have to modify this function.
        let args = GetArgs { key: key.clone() };
        let net = net::NetworkLocalHandle::current();
        for i in self.leader.load(Ordering::Relaxed).. {
            let i = i % self.servers.len();
            debug!("->{} {:?}", i, args);

            let call = net
                .clone()
                .call_owned::<_, GetReply>(self.servers[i], args.clone());
            match timeout(Duration::from_millis(500), call).await {
                Err(e) => {
                    debug!("<-{} {:?}", i, e);
                    continue;
                }
                Ok(Ok(GetReply::WrongLeader)) => {
                    debug!("<-{} wrong leader", i);
                    if i == self.servers.len() - 1 {
                        sleep(Duration::from_millis(100)).await;
                    }
                    continue;
                }
                Ok(Ok(GetReply::Ok { value })) => {
                    debug!("<-{} Get({:?})", i, key);
                    self.leader.store(i, Ordering::Relaxed);
                    return value;
                }
                Ok(e) => {
                    debug!("<-{} {:?}", i, e);
                    continue;
                }
            }
        }
        unreachable!()
    }

    pub async fn put(&self, key: String, value: String) {
        self.put_append(key, value, false).await;
    }

    pub async fn append(&self, key: String, value: String) {
        self.put_append(key, value, true).await;
    }

    /// shared by Put and Append.
    async fn put_append(&self, key: String, value: String, append: bool) {
        let args = PutAppendArgs {
            key,
            value,
            append,
            id: rand::rng().gen(),
        };
        let net = net::NetworkLocalHandle::current();
        for i in self.leader.load(Ordering::Relaxed).. {
            let i = i % self.servers.len();
            debug!("->{} {:?}", i, args);

            let call = net
                .clone()
                .call_owned::<_, PutAppendReply>(self.servers[i], args.clone());
            match timeout(Duration::from_millis(500), call).await {
                Err(e) => {
                    debug!("<-{} {:?}", i, e);
                    continue;
                }
                Ok(Ok(PutAppendReply::WrongLeader)) => {
                    debug!("<-{} wrong leader", i);
                    if i == self.servers.len() - 1 {
                        sleep(Duration::from_millis(100)).await;
                    }
                    continue;
                }
                Ok(Ok(PutAppendReply::Ok)) => {
                    debug!("<-{} ok", i);
                    self.leader.store(i, Ordering::Relaxed);
                    return;
                }
                Ok(e) => {
                    debug!("<-{} {:?}", i, e);
                    continue;
                }
            }
        }
    }
}
