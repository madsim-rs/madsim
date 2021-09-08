//! Asynchronous network endpoint and a controlled network simulator.

use self::network::{Mailbox, RecvMsg};
use bytes::{Buf, Bytes, BytesMut};
use futures::StreamExt;
use log::*;
use std::{
    collections::HashMap,
    io::{self, Write},
    net::{SocketAddr, ToSocketAddrs},
    sync::{Arc, Mutex},
};
use tokio::{
    io::AsyncWriteExt,
    net::{TcpListener, TcpStream},
    runtime::Handle,
    sync::{mpsc, oneshot},
};
use tokio_util::codec::{length_delimited::LengthDelimitedCodec, FramedRead, LinesCodec};

pub use self::network::{Config, Stat};
#[cfg(feature = "rpc")]
pub use self::rpc::Message;

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
    pub fn connect(&self, addr: SocketAddr) {
        todo!()
    }

    /// Disconnect a host from the network.
    pub fn disconnect(&self, addr: SocketAddr) {
        todo!()
    }

    /// Connect a pair of hosts.
    pub fn connect2(&self, addr1: SocketAddr, addr2: SocketAddr) {
        todo!()
    }

    /// Disconnect a pair of hosts.
    pub fn disconnect2(&self, addr1: SocketAddr, addr2: SocketAddr) {
        todo!()
    }
}

/// Local host network handle to the runtime.
#[derive(Clone)]
pub struct NetLocalHandle {
    addr: SocketAddr,
    sender: Arc<Mutex<Sender>>,
    mailbox: Arc<Mutex<Mailbox>>,
}

type Sender = HashMap<SocketAddr, mpsc::Sender<SendMsg>>;

struct SendMsg {
    tag: u64,
    data: Bytes,
    done: oneshot::Sender<()>,
}

impl NetLocalHandle {
    pub(crate) fn new(handle: &Handle, addr: SocketAddr) -> Self {
        let net = NetLocalHandle {
            addr,
            sender: Default::default(),
            mailbox: Default::default(),
        };
        let net0 = net.clone();
        // spawn acceptor
        handle.spawn(async move {
            let listener = TcpListener::bind(addr).await.unwrap();
            loop {
                let (stream, _) = listener.accept().await.unwrap();
                // receive real peer address
                let mut reader = FramedRead::new(stream, LinesCodec::new());
                let peer = reader
                    .next()
                    .await
                    .expect("connection closed")
                    .expect("failed to read peer address")
                    .parse::<SocketAddr>()
                    .expect("failed to parse peer address");
                let sender = net0.spawn(peer, reader.into_inner());
                net0.sender.lock().unwrap().insert(peer, sender);
            }
        });
        net
    }

    fn spawn(&self, peer: SocketAddr, stream: TcpStream) -> mpsc::Sender<SendMsg> {
        trace!("setup connection: {} -> {}", self.addr, peer);
        let (reader, mut writer) = stream.into_split();
        let mut reader = FramedRead::new(reader, LengthDelimitedCodec::new());
        let (sender, mut recver) = mpsc::channel(10);

        let mailbox = self.mailbox.clone();
        tokio::spawn(async move {
            while let Some(frame) = reader.next().await {
                let mut frame = frame.unwrap();
                let tag = frame.get_u64();
                mailbox.lock().unwrap().send(RecvMsg {
                    tag,
                    data: frame,
                    from: peer,
                });
            }
        });

        tokio::spawn(async move {
            while let Some(SendMsg { tag, data, done }) = recver.recv().await {
                let len = 8 + data.len();
                writer.write_u32(len as _).await.unwrap();
                writer.write_u64(tag).await.unwrap();
                writer.write(&data).await.unwrap();
                writer.flush().await.unwrap();
                done.send(()).unwrap();
            }
        });
        sender
    }

    fn get_or_connect(&self, addr: SocketAddr) -> mpsc::Sender<SendMsg> {
        self.sender
            .lock()
            .unwrap()
            .entry(addr)
            .or_insert_with(|| {
                let mut std_stream = std::net::TcpStream::connect(addr).unwrap();
                writeln!(std_stream, "{}", self.addr).unwrap();
                let stream = TcpStream::from_std(std_stream).unwrap();
                self.spawn(addr, stream)
            })
            .clone()
    }

    /// Returns a [`NetLocalHandle`] view over the currently running [`Runtime`].
    ///
    /// [`Runtime`]: crate::Runtime
    pub fn current() -> Self {
        crate::context::net_local_handle()
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
        trace!("send: {} -> {}, tag={}", self.addr, dst, tag);
        let sender = self.get_or_connect(dst);
        // Safety: sender task will refer the data until the `done` await return.
        let data = Bytes::from_static(unsafe { std::mem::transmute(data) });
        let (done, done_recver) = oneshot::channel();
        sender.send(SendMsg { tag, data, done }).await.ok().unwrap();
        done_recver.await.unwrap();
        Ok(())
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
        let len = buf.len().min(data.len());
        buf[..len].copy_from_slice(&data[..len]);
        Ok((len, from))
    }

    /// Receives a raw message.
    async fn recv_from_raw(&self, tag: u64) -> io::Result<(BytesMut, SocketAddr)> {
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
        let runtime = Runtime::new();
        let addr1 = "127.0.0.1:10000".parse().unwrap();
        let addr2 = "127.0.0.1:10001".parse().unwrap();
        let host1 = runtime.local_handle(addr1);
        let host2 = runtime.local_handle(addr2);

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

        runtime.block_on(f);
    }
}
