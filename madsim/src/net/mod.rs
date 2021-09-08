//! Asynchronous network endpoint and a controlled network simulator.
//!
//! # Examples
//!
//! ```
//! use madsim::{Runtime, net::NetLocalHandle};
//!
//! let runtime = Runtime::new();
//! let host1 = runtime.create_host("0.0.0.1:1").unwrap();
//! let host2 = runtime.create_host("0.0.0.2:1").unwrap();
//! let addr1 = host1.local_addr();
//! let addr2 = host2.local_addr();
//!
//! host1
//!     .spawn(async move {
//!         let net = NetLocalHandle::current();
//!         net.send_to(addr2, 1, &[1]).await.unwrap();
//!     })
//!     .detach();
//!
//! let f = host2.spawn(async move {
//!     let net = NetLocalHandle::current();
//!     let mut buf = vec![0; 0x10];
//!     let (len, from) = net.recv_from(1, &mut buf).await.unwrap();
//!     assert_eq!(from, addr1);
//!     assert_eq!(&buf[..len], &[1]);
//! });
//!
//! runtime.block_on(f);
//! ```

use log::*;
use std::{
    io,
    net::{SocketAddr, ToSocketAddrs},
    sync::{Arc, Mutex},
};

pub use self::network::{Config, Stat};
use self::network::{Network, Payload};
#[cfg(feature = "rpc")]
pub use self::rpc::Message;
use crate::{rand::*, time::*};

mod network;
#[cfg(feature = "rpc")]
mod rpc;

pub(crate) struct NetRuntime {
    handle: NetHandle,
}

impl NetRuntime {
    pub fn new(rand: RandHandle, time: TimeHandle) -> Self {
        let handle = NetHandle {
            network: Arc::new(Mutex::new(Network::new(rand.clone(), time.clone()))),
            rand,
            time,
        };
        NetRuntime { handle }
    }

    pub fn handle(&self) -> &NetHandle {
        &self.handle
    }
}

/// Network handle to the runtime.
#[derive(Clone)]
pub struct NetHandle {
    network: Arc<Mutex<Network>>,
    rand: RandHandle,
    time: TimeHandle,
}

impl NetHandle {
    /// Return a handle of the specified host.
    pub(crate) fn get_host(&self, addr: SocketAddr) -> NetLocalHandle {
        NetLocalHandle {
            handle: self.clone(),
            addr,
        }
    }

    /// Return a handle of the specified host.
    pub fn create_host(&self, addr: SocketAddr) -> NetLocalHandle {
        self.network.lock().unwrap().insert(addr);
        NetLocalHandle {
            handle: self.clone(),
            addr,
        }
    }

    /// Get the statistics.
    pub fn stat(&self) -> Stat {
        self.network.lock().unwrap().stat().clone()
    }

    /// Update network configurations.
    pub fn update_config(&self, f: impl FnOnce(&mut Config)) {
        let mut network = self.network.lock().unwrap();
        network.update_config(f);
    }

    /// Connect a host to the network.
    pub fn connect(&self, addr: SocketAddr) {
        let mut network = self.network.lock().unwrap();
        network.unclog(addr);
    }

    /// Disconnect a host from the network.
    pub fn disconnect(&self, addr: SocketAddr) {
        let mut network = self.network.lock().unwrap();
        network.clog(addr);
    }

    /// Connect a pair of hosts.
    pub fn connect2(&self, addr1: SocketAddr, addr2: SocketAddr) {
        let mut network = self.network.lock().unwrap();
        network.unclog_link(addr1, addr2);
        network.unclog_link(addr2, addr1);
    }

    /// Disconnect a pair of hosts.
    pub fn disconnect2(&self, addr1: SocketAddr, addr2: SocketAddr) {
        let mut network = self.network.lock().unwrap();
        network.clog_link(addr1, addr2);
        network.clog_link(addr2, addr1);
    }
}

/// Local host network handle to the runtime.
#[derive(Clone)]
pub struct NetLocalHandle {
    handle: NetHandle,
    addr: SocketAddr,
}

impl NetLocalHandle {
    /// Returns a [`NetLocalHandle`] view over the currently running [`Runtime`].
    ///
    /// [`Runtime`]: crate::Runtime
    pub fn current() -> Self {
        crate::context::net_local_handle()
    }

    /// Returns the local socket address.
    pub fn local_addr(&self) -> SocketAddr {
        self.addr
    }

    /// Sends data with tag on the socket to the given address.
    ///
    /// # Example
    /// ```
    /// use madsim::{Runtime, net::NetLocalHandle};
    ///
    /// Runtime::new().block_on(async {
    ///     let net = NetLocalHandle::current();
    ///     net.send_to("127.0.0.1:4242", 0, &[0; 10]).await.expect("couldn't send data");
    /// });
    /// ```
    pub async fn send_to(&self, dst: impl ToSocketAddrs, tag: u64, data: &[u8]) -> io::Result<()> {
        let dst = dst.to_socket_addrs()?.next().unwrap();
        self.send_to_raw(dst, tag, Box::new(Vec::from(data))).await
    }

    /// Receives a single message with given tag on the socket.
    /// On success, returns the number of bytes read and the origin.
    ///
    /// # Example
    /// ```no_run
    /// use madsim::{Runtime, net::NetLocalHandle};
    ///
    /// Runtime::new().block_on(async {
    ///     let net = NetLocalHandle::current();
    ///     let mut buf = [0; 10];
    ///     let (len, src) = net.recv_from(0, &mut buf).await.expect("couldn't receive data");
    /// });
    /// ```
    pub async fn recv_from(&self, tag: u64, buf: &mut [u8]) -> io::Result<(usize, SocketAddr)> {
        let (data, from) = self.recv_from_raw(tag).await?;
        // copy to buffer
        let data = data.downcast::<Vec<u8>>().expect("message is not data");
        let len = buf.len().min(data.len());
        buf[..len].copy_from_slice(&data[..len]);
        Ok((len, from))
    }

    /// Sends a raw message.
    async fn send_to_raw(&self, dst: SocketAddr, tag: u64, data: Payload) -> io::Result<()> {
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

    /// Receives a raw message.
    async fn recv_from_raw(&self, tag: u64) -> io::Result<(Payload, SocketAddr)> {
        let recver = self.handle.network.lock().unwrap().recv(self.addr, tag);
        let msg = recver.await.unwrap();
        // random delay
        let delay = Duration::from_micros(self.handle.rand.with(|rng| rng.gen_range(0..5)));
        self.handle.time.sleep(delay).await;

        trace!("recv: {} <- {}, tag={}", self.addr, msg.from, msg.tag);
        Ok((msg.data, msg.from))
    }
}

#[cfg(test)]
mod tests {
    use super::NetLocalHandle;
    use crate::{time::*, Runtime};

    #[test]
    fn send_recv() {
        let runtime = Runtime::new();
        let host1 = runtime.create_host("0.0.0.1:1").unwrap();
        let host2 = runtime.create_host("0.0.0.2:1").unwrap();
        let addr1 = host1.local_addr();
        let addr2 = host2.local_addr();

        host1
            .spawn(async move {
                let net = NetLocalHandle::current();
                net.send_to(addr2, 1, &[1]).await.unwrap();

                sleep(Duration::from_secs(1)).await;
                net.send_to(addr2, 2, &[2]).await.unwrap();
            })
            .detach();

        let f = host2.spawn(async move {
            let net = NetLocalHandle::current();
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
        let host1 = runtime.create_host("0.0.0.1:1").unwrap();
        let host2 = runtime.create_host("0.0.0.2:1").unwrap();
        let addr1 = host1.local_addr();
        let addr2 = host2.local_addr();

        host1
            .spawn(async move {
                let net = NetLocalHandle::current();
                sleep(Duration::from_secs(2)).await;
                net.send_to(addr2, 1, &[1]).await.unwrap();
            })
            .detach();

        let f = host2.spawn(async move {
            let net = NetLocalHandle::current();
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
