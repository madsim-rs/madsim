//! Tag-matching API over TCP.

use crate::task;
use bytes::{Buf, Bytes};
use futures_util::StreamExt;
use std::{
    collections::{hash_map::Entry, HashMap},
    io::{self, IoSlice},
    net::SocketAddr,
    sync::{Arc, Mutex, RwLock},
};
use tokio::{
    io::AsyncWriteExt,
    net::{lookup_host, TcpListener, TcpStream, ToSocketAddrs},
    sync::{mpsc, oneshot},
};
use tokio_util::codec::{length_delimited::LengthDelimitedCodec, FramedRead};
use tracing::*;

/// An endpoint.
#[derive(Clone)]
pub struct Endpoint {
    addr: SocketAddr,
    inner: Arc<Inner>,
}

#[derive(Default)]
struct Inner {
    sender: RwLock<Sender>,
    mailbox: Mutex<Mailbox>,
    tasks: Mutex<Vec<task::JoinHandle<()>>>,
}

type Sender = HashMap<SocketAddr, mpsc::Sender<SendMsg>>;

struct SendMsg {
    tag: u64,
    bufs: &'static mut [IoSlice<'static>],
    done: oneshot::Sender<()>,
}

impl Endpoint {
    /// Creates an [`Endpoint`] from the given address.
    pub async fn bind(addr: impl ToSocketAddrs) -> io::Result<Self> {
        let listener = TcpListener::bind(addr).await?;
        let addr = listener.local_addr()?;
        let ep = Endpoint {
            addr,
            inner: Default::default(),
        };
        trace!("new endpoint: {addr}");

        let inner = Arc::downgrade(&ep.inner);
        let acceptor = task::spawn(async move {
            loop {
                let (stream, _) = listener.accept().await.unwrap();
                if let Some(inner) = inner.upgrade() {
                    inner.setup_connection(addr, None, stream).await;
                } else {
                    return;
                }
            }
        });
        ep.inner.tasks.lock().unwrap().push(acceptor);
        Ok(ep)
    }
}

impl Inner {
    async fn setup_connection(
        self: &Arc<Self>,
        addr: SocketAddr,
        peer: Option<(SocketAddr, mpsc::Receiver<SendMsg>)>,
        stream: TcpStream,
    ) {
        stream.set_nodelay(true).expect("failed to set nodelay");
        let (reader, writer) = stream.into_split();
        let mut writer = tokio::io::BufWriter::new(writer);
        let mut reader = FramedRead::new(reader, LengthDelimitedCodec::new());
        let (peer, mut send_rx) = if let Some((peer, send_rx)) = peer {
            // send local address
            let addr_str = addr.to_string();
            writer.write_u32(addr_str.len() as _).await.unwrap();
            writer.write_all(addr_str.as_bytes()).await.unwrap();
            writer.flush().await.unwrap();
            (peer, send_rx)
        } else {
            // receive real peer address
            let data = reader
                .next()
                .await
                .expect("connection closed")
                .expect("failed to read peer address");
            let peer = std::str::from_utf8(&data)
                .expect("invalid utf8")
                .parse::<SocketAddr>()
                .expect("failed to parse peer address");
            // register sender
            let (send_tx, send_rx) = mpsc::channel(10);
            if self.sender.write().unwrap().insert(peer, send_tx).is_some() {
                warn!("duplicate connection: {addr} -> {peer}");
            }
            (peer, send_rx)
        };
        trace!("setup connection: {addr} -> {peer}");

        let sender_task = task::spawn(async move {
            while let Some(SendMsg {
                tag,
                mut bufs,
                done,
            }) = send_rx.recv().await
            {
                let len = 8 + bufs.iter().map(|s| s.len()).sum::<usize>();
                writer.write_u32(len as _).await.unwrap();
                writer.write_u64(tag).await.unwrap();
                while !bufs.is_empty() {
                    let n = writer.write_vectored(bufs).await.unwrap();
                    advance_slices(&mut bufs, n);
                }
                writer.flush().await.unwrap();
                done.send(()).unwrap();
            }
        });

        let inner = Arc::downgrade(self);
        let _recver_task = task::spawn(async move {
            while let Some(frame) = reader.next().await {
                let mut frame = match frame {
                    Ok(frame) => frame.freeze(),
                    Err(_) => break,
                };
                let tag = frame.get_u64();
                debug!(?addr, ?peer, tag, len = frame.len(), "recv");
                if let Some(inner) = inner.upgrade() {
                    inner.mailbox.lock().unwrap().deliver(RecvMsg {
                        tag,
                        data: frame,
                        from: peer,
                    });
                } else {
                    break;
                }
            }
            // NOTE: now the tcp connection is closed by remote.
            //       We need to abort the sender task to close the connection cleanly.
            sender_task.abort();
            debug!(?addr, ?peer, "close connection");
            if let Some(inner) = inner.upgrade() {
                inner.sender.write().unwrap().remove(&peer);
            }
        });

        // self.tasks
        //     .lock()
        //     .unwrap()
        //     .extend([sender_task, recver_task]);
    }
}

impl Endpoint {
    /// Get a sender to the remote endpoint `addr`. Setup connection if it doesn't exist.
    async fn get_or_connect(&self, addr: SocketAddr) -> mpsc::Sender<SendMsg> {
        // return sender if it exists
        if let Some(sender) = self.inner.sender.read().unwrap().get(&addr) {
            return sender.clone();
        }
        // create a new channel and register sender
        let (sender, recver) = match self.inner.sender.write().unwrap().entry(addr) {
            Entry::Occupied(e) => return e.get().clone(),
            Entry::Vacant(e) => {
                let (tx, rx) = mpsc::channel(10);
                e.insert(tx.clone());
                (tx, rx)
            }
        };
        // connect to the remote endpoint
        // trace!("{} try sending to {} but has not connected, start to connect actively", self.addr, addr);
        let stream = TcpStream::connect(addr).await.unwrap();
        self.inner
            .setup_connection(self.addr, Some((addr, recver)), stream)
            .await;
        sender
    }

    /// Returns the local socket address.
    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        Ok(self.addr)
    }

    /// Sends data with tag on the socket to the given address.
    ///
    /// # Example
    /// ```ignore
    /// # use madsim_std as madsim;
    /// use madsim::{Runtime, net::Endpoint};
    ///
    /// Runtime::new().block_on(async {
    ///     let net = Endpoint::bind("127.0.0.1:0").await.unwrap();
    ///     net.send_to("127.0.0.1:4242", 0, &[0; 10]).await.expect("couldn't send data");
    /// });
    /// ```
    pub async fn send_to(&self, dst: impl ToSocketAddrs, tag: u64, data: &[u8]) -> io::Result<()> {
        self.send_to_vectored(dst, tag, &mut [IoSlice::new(data)])
            .await
    }

    /// Like [`send_to`], except that it writes from a slice of buffers.
    ///
    /// [`send_to`]: Endpoint::send_to
    pub async fn send_to_vectored(
        &self,
        dst: impl ToSocketAddrs,
        tag: u64,
        bufs: &mut [IoSlice<'_>],
    ) -> io::Result<()> {
        let dst = lookup_host(dst).await?.next().unwrap();
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
    /// use madsim::{Runtime, net::Endpoint};
    ///
    /// Runtime::new().block_on(async {
    ///     let net = Endpoint::bind("127.0.0.1:0").await.unwrap();
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
        let recver = self.inner.mailbox.lock().unwrap().recv(tag);
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
    fn deliver(&mut self, msg: RecvMsg) {
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

    fn recv(&mut self, tag: u64) -> oneshot::Receiver<RecvMsg> {
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

// from std 1.60.0 `IoSlice::advance_slices`
fn advance_slices(bufs: &mut &mut [IoSlice<'_>], n: usize) {
    // Number of buffers to remove.
    let mut remove = 0;
    // Total length of all the to be removed buffers.
    let mut accumulated_len = 0;
    for buf in bufs.iter() {
        if accumulated_len + buf.len() > n {
            break;
        } else {
            accumulated_len += buf.len();
            remove += 1;
        }
    }

    *bufs = &mut std::mem::take(bufs)[remove..];
    if !bufs.is_empty() {
        // bufs[0].advance(n - accumulated_len)
        bufs[0] = IoSlice::new(unsafe { std::mem::transmute(&bufs[0][n - accumulated_len..]) });
    }
}
