//! Asynchronous network endpoint and a controlled network simulator.
//!
//! # Examples
//!
//! ```
//! use madsim_std::{Runtime, net::NetLocalHandle};
//!
//! let runtime = Runtime::new();
//! let host1 = runtime.create_host("127.0.0.1:0").unwrap();
//! let host2 = runtime.create_host("127.0.0.1:0").unwrap();
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

use self::network::{Mailbox, RecvMsg};
use bytes::Buf;
use futures::StreamExt;
use log::*;
use std::{
    collections::HashMap,
    io::{self, IoSlice},
    net::{SocketAddr, ToSocketAddrs},
    sync::{Arc, Mutex},
};
use tokio::{
    io::AsyncWriteExt,
    net::{TcpListener, TcpStream},
    runtime::Handle,
    sync::{mpsc, oneshot},
};
use tokio_util::codec::{length_delimited::LengthDelimitedCodec, FramedRead};

pub use self::network::{Config, Stat};
#[cfg(feature = "rpc")]
pub use self::rpc::Message;
pub use bytes::Bytes;

mod network;
#[cfg(feature = "rpc")]
mod rpc;

/// Network handle to the runtime.
#[derive(Clone)]
pub struct NetHandle {}

impl NetHandle {
    pub(crate) fn new() -> Self {
        NetHandle {}
    }

    /// Return a handle of the specified host.
    pub fn local_handle(&self, addr: SocketAddr) -> NetLocalHandle {
        todo!()
    }

    /// Get the statistics.
    pub fn stat(&self) -> Stat {
        Stat::default()
    }

    /// Update network configurations.
    pub fn update_config(&self, _f: impl FnOnce(&mut Config)) {}

    /// Connect a host to the network.
    pub fn connect(&self, _addr: SocketAddr) {}

    /// Disconnect a host from the network.
    pub fn disconnect(&self, _addr: SocketAddr) {}

    /// Connect a pair of hosts.
    pub fn connect2(&self, _addr1: SocketAddr, _addr2: SocketAddr) {}

    /// Disconnect a pair of hosts.
    pub fn disconnect2(&self, _addr1: SocketAddr, _addr2: SocketAddr) {}
}

/// Local host network handle to the runtime.
#[derive(Clone)]
pub struct NetLocalHandle {
    addr: SocketAddr,
    sender: Arc<tokio::sync::Mutex<Sender>>,
    mailbox: Arc<Mutex<Mailbox>>,
}

type Sender = HashMap<SocketAddr, mpsc::Sender<SendMsg>>;

struct SendMsg {
    tag: u64,
    bufs: &'static [IoSlice<'static>],
    done: oneshot::Sender<()>,
}

impl NetLocalHandle {
    pub(crate) fn new(handle: &Handle, addr: impl ToSocketAddrs) -> io::Result<Self> {
        let listener = std::net::TcpListener::bind(addr)?;
        listener.set_nonblocking(true)?;
        let net = NetLocalHandle {
            addr: listener.local_addr()?,
            sender: Default::default(),
            mailbox: Default::default(),
        };
        trace!("new host: {}", net.addr);
        let net0 = net.clone();
        // spawn acceptor
        handle.spawn(async move {
            let listener = TcpListener::from_std(listener).unwrap();
            loop {
                let (stream, _) = listener.accept().await.unwrap();
                let (peer, sender) = net0.spawn(None, stream).await;
                net0.sender.lock().await.insert(peer, sender);
            }
        });
        Ok(net)
    }

    async fn spawn(
        &self,
        peer: Option<SocketAddr>,
        stream: TcpStream,
    ) -> (SocketAddr, mpsc::Sender<SendMsg>) {
        let (reader, mut writer) = stream.into_split();
        let mut reader = FramedRead::new(reader, LengthDelimitedCodec::new());
        let (sender, mut recver) = mpsc::channel(10);
        let peer = if let Some(peer) = peer {
            // send local address
            let addr_str = self.addr.to_string();
            writer.write_u32(addr_str.len() as _).await.unwrap();
            writer.write_all(addr_str.as_bytes()).await.unwrap();
            writer.flush().await.unwrap();
            peer
        } else {
            // receive real peer address
            let data = reader
                .next()
                .await
                .expect("connection closed")
                .expect("failed to read peer address");
            std::str::from_utf8(&data)
                .expect("invalid utf8")
                .parse::<SocketAddr>()
                .expect("failed to parse peer address")
        };
        trace!("setup connection: {} -> {}", self.addr, peer);

        let mailbox = self.mailbox.clone();
        let addr = self.addr;
        tokio::spawn(async move {
            debug!("try recv: {} <- {}", addr, peer);
            while let Some(frame) = reader.next().await {
                let mut frame = frame.unwrap().freeze();
                let tag = frame.get_u64();
                mailbox.lock().unwrap().send(RecvMsg {
                    tag,
                    data: frame,
                    from: peer,
                });
            }
        });

        tokio::spawn(async move {
            while let Some(SendMsg { tag, bufs, done }) = recver.recv().await {
                let len = 8 + bufs.iter().map(|s| s.len()).sum::<usize>();
                writer.write_u32(len as _).await.unwrap();
                writer.write_u64(tag).await.unwrap();
                writer.write_vectored(bufs).await.unwrap();
                writer.flush().await.unwrap();
                done.send(()).unwrap();
            }
        });
        (peer, sender)
    }

    async fn get_or_connect(&self, addr: SocketAddr) -> mpsc::Sender<SendMsg> {
        let mut senders = self.sender.lock().await;
        if !senders.contains_key(&addr) {
            let stream = TcpStream::connect(addr).await.unwrap();
            let (_, sender) = self.spawn(Some(addr), stream).await;
            senders.insert(addr, sender);
        }
        senders[&addr].clone()
    }

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
    /// ```ignore
    /// use madsim_std::{Runtime, net::NetLocalHandle};
    ///
    /// Runtime::new().block_on(async {
    ///     let net = NetLocalHandle::current();
    ///     net.send_to("127.0.0.1:4242", 0, &[0; 10]).await.expect("couldn't send data");
    /// });
    /// ```
    pub async fn send_to(&self, dst: impl ToSocketAddrs, tag: u64, data: &[u8]) -> io::Result<()> {
        self.send_to_vectored(dst, tag, &[IoSlice::new(data)]).await
    }

    /// Like [`send_to`], except that it writes from a slice of buffers.
    ///
    /// [`send_to`]: NetLocalHandle::send_to
    pub async fn send_to_vectored(
        &self,
        dst: impl ToSocketAddrs,
        tag: u64,
        bufs: &[IoSlice<'_>],
    ) -> io::Result<()> {
        let dst = dst.to_socket_addrs()?.next().unwrap();
        trace!("send: {} -> {}, tag={}", self.addr, dst, tag);
        let sender = self.get_or_connect(dst).await;
        // Safety: sender task will refer the data until the `done` await return.
        let bufs = unsafe { std::mem::transmute(bufs) };
        let (done, done_recver) = oneshot::channel();
        sender.send(SendMsg { tag, bufs, done }).await.ok().unwrap();
        done_recver.await.unwrap();
        Ok(())
    }

    /// Receives a single message with given tag on the socket.
    /// On success, returns the number of bytes read and the origin.
    ///
    /// # Example
    /// ```ignore
    /// use madsim_std::{Runtime, net::NetLocalHandle};
    ///
    /// Runtime::new().block_on(async {
    ///     let net = NetLocalHandle::current();
    ///     let mut buf = [0; 10];
    ///     let (len, src) = net.recv_from(0, &mut buf).await.expect("couldn't receive data");
    /// });
    /// ```
    pub async fn recv_from(&self, tag: u64, buf: &mut [u8]) -> io::Result<(usize, SocketAddr)> {
        let (data, from) = self.recv_from_raw(tag).await?;
        let len = buf.len().min(data.len());
        buf[..len].copy_from_slice(&data[..len]);
        Ok((len, from))
    }

    /// Receives a raw message.
    async fn recv_from_raw(&self, tag: u64) -> io::Result<(Bytes, SocketAddr)> {
        let recver = self.mailbox.lock().unwrap().recv(tag);
        let msg = recver.await.unwrap();
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
        let rt = Runtime::new();
        let host1 = rt.create_host("127.0.0.1:0").unwrap();
        let host2 = rt.create_host("127.0.0.1:0").unwrap();
        let addr1 = host1.local_addr();
        let addr2 = host2.local_addr();

        host1
            .spawn(async move {
                let net = NetLocalHandle::current();
                net.send_to(addr2, 1, &[1]).await.unwrap();

                sleep(Duration::from_millis(10)).await;
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

        rt.block_on(f);
    }
}
