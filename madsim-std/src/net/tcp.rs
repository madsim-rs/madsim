//! Tag-matching API over TCP.

use super::network::{Config, Stat};
use bytes::{Buf, Bytes};
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
    runtime::Runtime,
    sync::{mpsc, oneshot},
    task::LocalSet,
};
use tokio_util::codec::{length_delimited::LengthDelimitedCodec, FramedRead};

/// Network handle to the runtime.
#[derive(Clone)]
pub struct NetHandle {}

impl NetHandle {
    pub(crate) fn new() -> Self {
        NetHandle {}
    }

    /// Create a host which will be bound to the specified address.
    pub(crate) fn create_host(
        &self,
        rt: &Runtime,
        local: &LocalSet,
        addr: impl ToSocketAddrs,
    ) -> io::Result<NetLocalHandle> {
        NetLocalHandle::new(rt, local, addr)
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
    fn new(rt: &Runtime, _local: &LocalSet, addr: impl ToSocketAddrs) -> io::Result<Self> {
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
        rt.spawn(async move {
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
        let (reader, writer) = stream.into_split();
        let mut writer = tokio::io::BufWriter::new(writer);
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
        if let std::collections::hash_map::Entry::Vacant(e) = senders.entry(addr) {
            let stream = TcpStream::connect(addr).await.unwrap();
            let (_, sender) = self.spawn(Some(addr), stream).await;
            e.insert(sender);
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
    /// # use madsim_std as madsim;
    /// use madsim::{Runtime, net::NetLocalHandle};
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
    /// # use madsim_std as madsim;
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
    pub(crate) async fn recv_from_raw(&self, tag: u64) -> io::Result<(Bytes, SocketAddr)> {
        let recver = self.mailbox.lock().unwrap().recv(tag);
        let msg = recver.await.unwrap();
        trace!("recv: {} <- {}, tag={}", self.addr, msg.from, msg.tag);
        Ok((msg.data, msg.from))
    }
}

#[derive(Debug)]
struct RecvMsg {
    tag: u64,
    data: Bytes,
    from: SocketAddr,
}

#[derive(Default)]
struct Mailbox {
    registered: Vec<(u64, oneshot::Sender<RecvMsg>)>,
    msgs: Vec<RecvMsg>,
}

impl Mailbox {
    pub fn send(&mut self, msg: RecvMsg) {
        let mut i = 0;
        let mut msg = Some(msg);
        while i < self.registered.len() {
            if matches!(&msg, Some(msg) if msg.tag == self.registered[i].0) {
                // tag match, take and try send
                let (_, sender) = self.registered.swap_remove(i);
                msg = match sender.send(msg.take().unwrap()) {
                    Ok(_) => return,
                    Err(m) => Some(m),
                };
                // failed to send, try next
            } else {
                // tag mismatch, move to next
                i += 1;
            }
        }
        // failed to match awaiting recv, save
        self.msgs.push(msg.unwrap());
    }

    pub fn recv(&mut self, tag: u64) -> oneshot::Receiver<RecvMsg> {
        let (tx, rx) = oneshot::channel();
        if let Some(idx) = self.msgs.iter().position(|msg| tag == msg.tag) {
            let msg = self.msgs.swap_remove(idx);
            tx.send(msg).ok().unwrap();
        } else {
            self.registered.push((tag, tx));
        }
        rx
    }
}
