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

    pub fn set_packet_loss_rate(&self, rate: f64) {
        let mut network = self.network.lock().unwrap();
        network.update_config(|cfg| cfg.packet_loss_rate = rate);
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
        let delay = Duration::from_micros(self.handle.rand.clone().gen_range(0..5));
        self.handle.time.sleep(delay).await;
        Ok(())
    }

    pub async fn recv_from(&self, tag: u64, data: &mut [u8]) -> io::Result<(usize, SocketAddr)> {
        let recver = self.handle.network.lock().unwrap().recv(self.addr, tag);
        let msg = recver.await.unwrap();
        // copy to buffer
        let len = data.len().min(msg.data.len());
        data[..len].copy_from_slice(&msg.data[..len]);
        // random delay
        let delay = Duration::from_micros(self.handle.rand.clone().gen_range(0..5));
        self.handle.time.sleep(delay).await;

        trace!(
            "recv: {} <- {}, tag={}, len={}",
            self.addr,
            msg.from,
            msg.tag,
            len
        );
        Ok((len, msg.from))
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
}
