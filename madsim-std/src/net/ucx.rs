//! Tag-matching API over UCX.

use async_ucx::ucp::{Context, Endpoint};
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
use tokio::{
    runtime::Runtime,
    sync::{mpsc, oneshot},
    task::LocalSet,
};

/// Network handle to the runtime.
#[derive(Clone)]
pub(crate) struct NetHandle {
    context: Arc<Context>,
}

impl NetHandle {
    pub(crate) fn new() -> Self {
        let context = Context::new().unwrap();
        NetHandle { context }
    }

    pub(crate) fn create_host(
        &self,
        rt: &Runtime,
        local: &LocalSet,
        addr: impl ToSocketAddrs,
    ) -> io::Result<NetLocalHandle> {
        NetLocalHandle::new(rt, local, &self.context, addr)
    }
}

/// Local host network handle to the runtime.
#[derive(Clone)]
pub struct NetLocalHandle {
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

impl NetLocalHandle {
    fn new(
        rt: &Runtime,
        local: &LocalSet,
        context: &Arc<Context>,
        addr: impl ToSocketAddrs,
    ) -> io::Result<Self> {
        let context = context.clone();
        let addr = addr.to_socket_addrs()?.next().unwrap();
        let (sender, mut send_recver) = mpsc::channel::<SendMsg>(8);
        let (recver, mut recv_recver) = mpsc::channel::<RecvMsg>(8);

        // all UCX operations need to be done in a single thread
        let addr = local.block_on(rt, async move {
            let worker = context.create_worker().unwrap();
            // polling net events
            tokio::task::spawn_local(worker.clone().event_poll());

            // collection of all endpoints
            // shared by sender task and listener task.
            let eps: Rc<RefCell<HashMap<SocketAddr, Rc<Endpoint>>>> = Default::default();

            let mut listener = worker.create_listener(addr).unwrap();
            let addr = listener.socket_addr().unwrap();
            // spawn a task to accept connections
            tokio::task::spawn_local({
                let worker = worker.clone();
                let eps = eps.clone();
                async move {
                    loop {
                        let conn = listener.next().await;
                        let peer = conn.remote_addr().unwrap();
                        let ep = Rc::new(worker.accept(conn).unwrap());
                        eps.borrow_mut().insert(peer, ep);
                    }
                }
            });

            // spawn a task to send messages
            tokio::task::spawn_local({
                let worker = worker.clone();
                async move {
                    while let Some(msg) = send_recver.recv().await {
                        let ep = eps
                            .borrow_mut()
                            .entry(msg.dst)
                            // setup connection first if the endpoint does not exist.
                            .or_insert_with(|| Rc::new(worker.connect(msg.dst).unwrap()))
                            .clone();
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
            tokio::task::spawn_local(async move {
                while let Some(RecvMsg { tag, bufs, done }) = recv_recver.recv().await {
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
            });
            addr
        });

        trace!("new host: {}", addr);
        Ok(NetLocalHandle {
            addr,
            sender,
            recver,
        })
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
    /// use madsim_std::{Runtime, net::NetLocalHandle};
    ///
    /// Runtime::new().block_on(async {
    ///     let net = NetLocalHandle::current();
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
