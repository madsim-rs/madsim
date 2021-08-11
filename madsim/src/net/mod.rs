use log::*;
use std::{
    io,
    net::SocketAddr,
    sync::{Arc, Mutex},
};

use self::network::Network;
pub use self::network::{Config, Stat};
use crate::{rand::*, time::*};

mod network;
#[cfg(feature = "rpc")]
mod rpc;

pub struct NetworkRuntime {
    handle: NetworkHandle,
}

impl NetworkRuntime {
    pub(crate) fn new(rand: RandomHandle, time: TimeHandle) -> Self {
        let handle = NetworkHandle {
            network: Arc::new(Mutex::new(Network::new(rand.clone(), time.clone()))),
            rand,
            time,
        };
        NetworkRuntime { handle }
    }

    pub fn handle(&self) -> &NetworkHandle {
        &self.handle
    }
}

#[derive(Clone)]
pub struct NetworkHandle {
    network: Arc<Mutex<Network>>,
    rand: RandomHandle,
    time: TimeHandle,
}

impl NetworkHandle {
    pub fn local_handle(&self, addr: SocketAddr) -> NetworkLocalHandle {
        self.network.lock().unwrap().insert(addr);
        NetworkLocalHandle {
            handle: self.clone(),
            addr,
        }
    }

    pub fn stat(&self) -> Stat {
        self.network.lock().unwrap().stat().clone()
    }

    pub fn update_config(&self, f: impl FnOnce(&mut Config)) {
        let mut network = self.network.lock().unwrap();
        network.update_config(f);
    }

    pub fn connect(&self, addr: SocketAddr) {
        let mut network = self.network.lock().unwrap();
        network.insert(addr);
        network.unclog(addr);
    }

    pub fn disconnect(&self, addr: SocketAddr) {
        let mut network = self.network.lock().unwrap();
        network.clog(addr);
    }

    pub fn connect2(&self, addr1: SocketAddr, addr2: SocketAddr) {
        let mut network = self.network.lock().unwrap();
        network.insert(addr1);
        network.insert(addr2);
        network.unclog_link(addr1, addr2);
        network.unclog_link(addr2, addr1);
    }

    pub fn disconnect2(&self, addr1: SocketAddr, addr2: SocketAddr) {
        let mut network = self.network.lock().unwrap();
        network.clog_link(addr1, addr2);
        network.clog_link(addr2, addr1);
    }
}

#[derive(Clone)]
pub struct NetworkLocalHandle {
    handle: NetworkHandle,
    addr: SocketAddr,
}

impl NetworkLocalHandle {
    pub fn current() -> Self {
        crate::context::net_local_handle()
    }

    pub async fn send_to(&self, dst: SocketAddr, tag: u64, data: &[u8]) -> io::Result<()> {
        self.handle
            .network
            .lock()
            .unwrap()
            .send(self.addr, dst, tag, data);
        // random delay
        let delay = Duration::from_micros(self.handle.rand.with(|rng| rng.gen_range(0..5)));
        self.handle.time.sleep(delay).await;
        Ok(())
    }

    pub async fn recv_from(&self, tag: u64, buf: &mut [u8]) -> io::Result<(usize, SocketAddr)> {
        let (data, from) = self.recv_from_vec(tag).await?;
        // copy to buffer
        let len = buf.len().min(data.len());
        buf[..len].copy_from_slice(&data[..len]);
        Ok((len, from))
    }

    async fn recv_from_vec(&self, tag: u64) -> io::Result<(Vec<u8>, SocketAddr)> {
        let recver = self.handle.network.lock().unwrap().recv(self.addr, tag);
        let msg = recver.await.unwrap();
        // random delay
        let delay = Duration::from_micros(self.handle.rand.with(|rng| rng.gen_range(0..5)));
        self.handle.time.sleep(delay).await;

        trace!(
            "recv: {} <- {}, tag={}, len={}",
            self.addr,
            msg.from,
            msg.tag,
            msg.data.len(),
        );
        Ok((msg.data, msg.from))
    }
}

#[cfg(test)]
mod tests {
    use super::NetworkLocalHandle;
    use crate::{time::*, Runtime};

    #[test]
    fn send_recv() {
        let runtime = Runtime::new();
        let addr1 = "0.0.0.1:1".parse().unwrap();
        let addr2 = "0.0.0.2:1".parse().unwrap();
        let host1 = runtime.local_handle(addr1);
        let host2 = runtime.local_handle(addr2);

        host1
            .spawn(async move {
                let net = NetworkLocalHandle::current();
                net.send_to(addr2, 1, &[1]).await.unwrap();

                sleep(Duration::from_secs(1)).await;
                net.send_to(addr2, 2, &[2]).await.unwrap();
            })
            .detach();

        let f = host2.spawn(async move {
            let net = NetworkLocalHandle::current();
            let mut buf = vec![0; 0x10];
            let (len, from) = net.recv_from(2, &mut buf).await.unwrap();
            assert_eq!(len, 1);
            assert_eq!(from, addr1);
            assert_eq!(buf[0], 2);
            let (len, from) = net.recv_from(1, &mut buf).await.unwrap();
            assert_eq!(len, 1);
            assert_eq!(from, addr1);
            assert_eq!(buf[0], 1);
        });

        runtime.block_on(f);
    }

    #[test]
    fn receiver_drop() {
        let runtime = Runtime::new();
        let addr1 = "0.0.0.1:1".parse().unwrap();
        let addr2 = "0.0.0.2:1".parse().unwrap();
        let host1 = runtime.local_handle(addr1);
        let host2 = runtime.local_handle(addr2);

        host1
            .spawn(async move {
                let net = NetworkLocalHandle::current();
                sleep(Duration::from_secs(2)).await;
                net.send_to(addr2, 1, &[1]).await.unwrap();
            })
            .detach();

        let f = host2.spawn(async move {
            let net = NetworkLocalHandle::current();
            let mut buf = vec![0; 0x10];
            timeout(Duration::from_secs(1), net.recv_from(1, &mut buf))
                .await
                .err()
                .unwrap();
            // timeout and receiver dropped here
            sleep(Duration::from_secs(2)).await;
            // receive again should success
            let (len, from) = net.recv_from(1, &mut buf).await.unwrap();
            assert_eq!(len, 1);
            assert_eq!(from, addr1);
        });

        runtime.block_on(f);
    }
}
