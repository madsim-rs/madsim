use log::*;
use std::{
    io,
    net::SocketAddr,
    sync::{Arc, Mutex},
};

pub use self::network::Config;
use self::network::{Message, Network};
use crate::{executor::Spawner, rand::RandomHandle, time::TimeHandle};

mod network;

pub struct NetworkRuntime {
    network: Arc<Mutex<Network>>,
}

impl NetworkRuntime {
    pub fn new(rand: RandomHandle, time: TimeHandle, spawner: Spawner) -> Self {
        let config = Config::default();
        let network = Network::new(rand, time, spawner, config);
        NetworkRuntime {
            network: Arc::new(Mutex::new(network)),
        }
    }

    pub fn handle(&self, addr: SocketAddr) -> NetworkHandle {
        let recver = self.network.lock().unwrap().insert(addr);
        NetworkHandle {
            network: self.network.clone(),
            addr,
            recver,
        }
    }
}

#[derive(Clone)]
pub struct NetworkHandle {
    network: Arc<Mutex<Network>>,
    addr: SocketAddr,
    recver: async_channel::Receiver<Message>,
}

impl NetworkHandle {
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
            "recv: {}<-{}, tag={}, len={}",
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
    use super::*;
    use crate::Runtime;

    #[test]
    fn connect() {
        env_logger::init();

        let rt = Runtime::new().unwrap();
        let addr1 = "0.0.0.1:1".parse().unwrap();
        let addr2 = "0.0.0.2:1".parse().unwrap();
        let host1 = rt.net.handle(addr1);
        let host2 = rt.net.handle(addr2);

        rt.block_on(async move {
            host1.send_to(addr2, 1, &[1]).await.unwrap();
            let mut buf = vec![0; 0x10];
            let (len, tag, from) = host2.recv_from(&mut buf).await.unwrap();
            assert_eq!(len, 1);
            assert_eq!(tag, 1);
            assert_eq!(from, addr1);
        });
    }
}
