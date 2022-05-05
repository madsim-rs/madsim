//! Tag-matching API over UCX.

use async_ucx::ucp;
use bytes::Bytes;
use log::*;
use std::{
    cell::RefCell,
    collections::HashMap,
    io::{self, IoSlice, IoSliceMut},
    mem::MaybeUninit,
    net::{SocketAddr, ToSocketAddrs},
    rc::Rc,
    sync::Arc,
};
use tokio::sync::{mpsc, oneshot};

lazy_static::lazy_static! {
    static ref CONTEXT: Arc<ucp::Context> = ucp::Context::new().expect("failed to initialize UCX context");
}

/// An endpoint.
pub struct Endpoint {
    addr: SocketAddr,
    /// Client sends message from here.
    sender: mpsc::Sender<SendMsg>,
    /// Client receives message from here.
    recver: mpsc::Sender<RecvMsg>,
}

struct SendMsg {
    dst: SocketAddr,
    tag: u64,
    bufs: &'static [IoSlice<'static>],
    done: oneshot::Sender<()>,
}

struct RecvMsg {
    tag: u64,
    bufs: &'static mut [IoSliceMut<'static>],
    done: oneshot::Sender<(usize, SocketAddr)>,
}

fn worker_thread(
    addr: SocketAddr,
    addr_tx: oneshot::Sender<SocketAddr>,
    mut send_rx: mpsc::Receiver<SendMsg>,
    mut recv_rx: mpsc::Receiver<RecvMsg>,
) {
    // all UCX operations need to be done in a single thread
    let context = CONTEXT.clone();
    let worker = context.create_worker().unwrap();

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let local = tokio::task::LocalSet::new();

    // collection of all endpoints
    // shared by sender task and listener task.
    let eps: Rc<RefCell<HashMap<SocketAddr, Rc<ucp::Endpoint>>>> = Default::default();

    let mut listener = worker.create_listener(addr).unwrap();
    let addr = listener.socket_addr().unwrap();
    addr_tx.send(addr).unwrap();

    // spawn a task to accept connections
    local.spawn_local({
        let worker = worker.clone();
        let eps = eps.clone();
        async move {
            loop {
                let conn = listener.next().await;
                let peer = conn.remote_addr().unwrap();
                let ep = Rc::new(worker.accept(conn).await.unwrap());
                eps.borrow_mut().insert(peer, ep);
            }
        }
    });

    // spawn a task to send messages
    local.spawn_local({
        let worker = worker.clone();
        async move {
            while let Some(msg) = send_rx.recv().await {
                let mut eps = eps.borrow_mut();
                let ep = if let Some(ep) = eps.get(&msg.dst) {
                    ep.clone()
                } else {
                    // setup connection first if the endpoint does not exist.
                    let ep = Rc::new(worker.connect_socket(msg.dst).await.unwrap());
                    eps.insert(msg.dst, ep.clone());
                    ep
                };
                tokio::task::spawn_local(async move {
                    // insert 6 bytes src addr at front
                    let addr = socket_addr_to_bytes(addr);
                    let mut bufs = vec![IoSlice::new(&addr)];
                    bufs.extend_from_slice(msg.bufs);
                    ep.tag_send_vectored(msg.tag, &bufs).await.unwrap();
                    msg.done.send(()).unwrap();
                });
            }
        }
    });

    // spawn a task to receive messages
    local.spawn_local({
        let worker = worker.clone();
        async move {
            while let Some(RecvMsg { tag, bufs, done }) = recv_rx.recv().await {
                let worker = worker.clone();
                tokio::task::spawn_local(async move {
                    // extract 6 bytes src addr at front
                    let mut addr_buf = [0u8; 6];
                    let mut bufs1 = vec![IoSliceMut::new(&mut addr_buf)];
                    bufs1.reserve(bufs.len());
                    // safety: `bufs` will be moved to the new iovec
                    unsafe {
                        std::ptr::copy_nonoverlapping(
                            bufs.as_ptr(),
                            bufs1.as_mut_ptr().add(1),
                            bufs.len(),
                        );
                        bufs1.set_len(bufs.len() + 1);
                    }
                    let len = worker.tag_recv_vectored(tag, &mut bufs1).await.unwrap();
                    let addr = socket_addr_from_bytes(addr_buf);
                    done.send((len - 6, addr)).unwrap();
                });
            }
        }
    });

    // polling net events
    local.block_on(&rt, worker.event_poll()).unwrap();
}

impl Endpoint {
    /// Creates a [`Endpoint`] from the given address.
    pub async fn bind(addr: impl ToSocketAddrs) -> io::Result<Self> {
        let addr = addr.to_socket_addrs()?.next().unwrap();
        let (addr_tx, addr_rx) = oneshot::channel();
        let (sender, send_rx) = mpsc::channel::<SendMsg>(8);
        let (recver, recv_rx) = mpsc::channel::<RecvMsg>(8);
        std::thread::spawn(move || worker_thread(addr, addr_tx, send_rx, recv_rx));
        let addr = addr_rx.await.unwrap();
        trace!("new ep: {addr}");
        Ok(Endpoint {
            addr,
            sender,
            recver,
        })
    }

    /// Returns the local socket address.
    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        Ok(self.addr)
    }

    /// Sends data with tag on the socket to the given address.
    ///
    /// # Example
    /// ```ignore
    /// use madsim_std::{Runtime, net::Endpoint};
    ///
    /// Runtime::new().block_on(async {
    ///     let net = Endpoint::current();
    ///     net.send_to("127.0.0.1:4242", 0, &[0; 10]).await.expect("couldn't send data");
    /// });
    /// ```
    pub async fn send_to(&self, dst: impl ToSocketAddrs, tag: u64, data: &[u8]) -> io::Result<()> {
        self.send_to_vectored(dst, tag, &[IoSlice::new(data)]).await
    }

    /// Like [`send_to`], except that it writes from a slice of buffers.
    ///
    /// [`send_to`]: Endpoint::send_to
    pub async fn send_to_vectored(
        &self,
        dst: impl ToSocketAddrs,
        tag: u64,
        bufs: &[IoSlice<'_>],
    ) -> io::Result<()> {
        let dst = dst.to_socket_addrs()?.next().unwrap();
        trace!("send: {} -> {}, tag={}", self.addr, dst, tag);
        // Safety: sender task will refer the data until the `done` await return.
        let bufs = unsafe { std::mem::transmute(bufs) };
        let (done, done_recver) = oneshot::channel();
        let msg = SendMsg {
            dst,
            tag,
            bufs,
            done,
        };
        self.sender.send(msg).await.ok().unwrap();
        done_recver.await.unwrap();
        Ok(())
    }

    /// Receives a single message with given tag on the socket.
    /// On success, returns the number of bytes read and the origin.
    ///
    /// # Example
    /// ```ignore
    /// use madsim_std::{Runtime, net::Endpoint};
    ///
    /// Runtime::new().block_on(async {
    ///     let net = Endpoint::current();
    ///     let mut buf = [0; 10];
    ///     let (len, src) = net.recv_from(0, &mut buf).await.expect("couldn't receive data");
    /// });
    /// ```
    pub async fn recv_from(&self, tag: u64, buf: &mut [u8]) -> io::Result<(usize, SocketAddr)> {
        self.recv_from_vectored(tag, &mut [IoSliceMut::new(buf)])
            .await
    }

    pub async fn recv_from_vectored(
        &self,
        tag: u64,
        bufs: &mut [IoSliceMut<'_>],
    ) -> io::Result<(usize, SocketAddr)> {
        // Safety: recver task will refer the data until the `done` await return.
        let bufs = unsafe { std::mem::transmute(bufs) };
        let (done, done_recver) = oneshot::channel();
        let msg = RecvMsg { tag, bufs, done };
        self.recver.send(msg).await.ok().unwrap();
        let (len, from) = done_recver.await.unwrap();
        trace!("recv: {} <- {}, tag={}", self.addr, from, tag);
        Ok((len, from))
    }

    /// Receives a raw message.
    pub(crate) async fn recv_from_raw(&self, tag: u64) -> io::Result<(Bytes, SocketAddr)> {
        let buf = vec![MaybeUninit::<u8>::uninit(); 0x1000];
        let mut buf: Vec<u8> = unsafe { std::mem::transmute(buf) };
        let (len, from) = self.recv_from(tag, &mut buf).await?;
        Ok((Bytes::from(buf).split_to(len), from))
    }
}

fn socket_addr_to_bytes(addr: SocketAddr) -> [u8; 6] {
    match addr {
        SocketAddr::V4(addr) => {
            let [a, b, c, d] = addr.ip().octets();
            let [e, f] = addr.port().to_be_bytes();
            [a, b, c, d, e, f]
        }
        SocketAddr::V6(_) => todo!(),
    }
}

fn socket_addr_from_bytes(bytes: [u8; 6]) -> SocketAddr {
    let [a, b, c, d, e, f] = bytes;
    SocketAddr::from(([a, b, c, d], u16::from_be_bytes([e, f])))
}
