use log::*;
use std::{
    io,
    net::SocketAddr,
    sync::{Arc, Mutex},
};

pub use self::network::{Config, Stat};
use self::network::{Message, Network};
use crate::{rand::RandomHandle, time::TimeHandle};

mod network;

pub struct NetworkRuntime {
    handle: NetworkHandle,
}

impl NetworkRuntime {
    pub(crate) fn new(rand: RandomHandle, time: TimeHandle) -> Self {
        let handle = NetworkHandle {
            network: Arc::new(Mutex::new(Network::new(rand, time))),
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
}

impl NetworkHandle {
    pub fn local_handle(&self, addr: SocketAddr) -> NetworkLocalHandle {
        let recver = self.network.lock().unwrap().get(addr);
        NetworkLocalHandle {
            network: self.network.clone(),
            addr,
            recver,
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
        network.get(addr);
        network.unclog(addr);
    }

    pub fn disconnect(&self, addr: SocketAddr) {
        let mut network = self.network.lock().unwrap();
        network.clog(addr);
    }
}

#[derive(Clone)]
pub struct NetworkLocalHandle {
    network: Arc<Mutex<Network>>,
    addr: SocketAddr,
    recver: async_channel::Receiver<Message>,
}

impl NetworkLocalHandle {
    pub fn current() -> Self {
        crate::context::net_local_handle()
    }

    // pub async fn connect(&self, dst: SocketAddr) -> io::Result<Endpoint> {
    //     todo!()
    // }

    // pub async fn accept(&self) -> io::Result<(Endpoint, SocketAddr)> {
    //     todo!()
    // }

    pub async fn send_to(&self, dst: SocketAddr, tag: u64, data: &[u8]) -> io::Result<()> {
        self.network.lock().unwrap().send(self.addr, dst, tag, data);
        Ok(())
    }

    pub async fn recv_from(&self, data: &mut [u8]) -> io::Result<(usize, u64, SocketAddr)> {
        let msg = self.recver.recv().await.unwrap();
        let len = data.len().min(msg.data.len());
        data[..len].copy_from_slice(&msg.data[..len]);
        trace!(
            "recv: {} <- {}, tag={}, len={}",
            self.addr,
            msg.from,
            msg.tag,
            len
        );
        Ok((len, msg.tag, msg.from))
    }
}

#[cfg(test)]
mod tests {
    use super::NetworkLocalHandle;
    use crate::Runtime;

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
            })
            .detach();

        let f = host2.spawn(async move {
            let net = NetworkLocalHandle::current();
            let mut buf = vec![0; 0x10];
            let (len, tag, from) = net.recv_from(&mut buf).await.unwrap();
            assert_eq!(len, 1);
            assert_eq!(tag, 1);
            assert_eq!(from, addr1);
            assert_eq!(buf[0], 1);
        });

        runtime.block_on(f);
    }
}
