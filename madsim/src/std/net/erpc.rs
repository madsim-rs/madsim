use super::rpc::Bytes;
use crate::task;
use bytes::{BufMut, BytesMut};
use mad_rpc::{
    transport::{self, Transport},
    ud::VerbsTransport,
};
use std::{
    collections::{HashMap, VecDeque},
    io::{self, IoSlice, Write},
    net::SocketAddr,
    sync::{Arc, Mutex},
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{lookup_host, TcpListener, TcpStream, ToSocketAddrs},
    sync::{oneshot, Mutex as AsyncMutex},
};

// The origin rpc implementation in mad_rpc is not suitable for porting to madsim directly.
// The main logic is inspired by [`mad_rpc::rpc`] and [`madsim::net::tcp`]

#[derive(Clone)]
pub struct Endpoint {
    inner: Option<Arc<Inner>>,
    init_lock: Arc<AsyncMutex<Option<SocketAddr>>>,
}

struct Inner {
    addr: SocketAddr,
    transport: Mutex<VerbsTransport<Endpoint>>,
    /// mappings from SocketAddr to EndpointID in erpc
    mappings: AsyncMutex<HashMap<SocketAddr, u32>>,
    tasks: Mutex<Vec<task::JoinHandle<()>>>,
    msg_buf: Mutex<MsgBuffer>,
}

#[derive(Default)]
struct MsgBuffer {
    registered: HashMap<u64, VecDeque<oneshot::Sender<RecvMsg>>>,
    msgs: HashMap<u64, VecDeque<RecvMsg>>,
}

#[derive(Debug)]
struct MsgHeader {
    tag: u64,
    data_len: u32,
    //todo: need a better solution to send the socket addr
    from: SocketAddr,
}

#[derive(Debug)]
pub struct SendMsg<'a> {
    // Option here is used for detecting whether unpack msg has started
    header: Option<MsgHeader>,
    bufs: &'a mut [IoSlice<'a>],
}

#[derive(Debug)]
pub struct RecvMsg {
    #[allow(dead_code)]
    ep_id: u32,
    header: MsgHeader,
    // Option here is used for detecting whether unpack msg has started
    data: Option<BytesMut>,
}

fn mad_rpc_err_to_io_err(err: mad_rpc::Error) -> io::Error {
    io::Error::new(io::ErrorKind::Other, format!("{err:?}"))
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

impl MsgHeader {
    fn new(tag: u64, data_len: u32, from: SocketAddr) -> Self {
        MsgHeader {
            tag,
            data_len,
            from,
        }
    }

    // format: [tag:u64 , data_len:u32, from_len:u32, from_bytes:]
    fn serialize(&self, mut buf: &mut [u8]) -> usize {
        let mut len = 0;
        len += buf.write(&self.tag.to_be_bytes()).unwrap();
        len += buf.write(&self.data_len.to_be_bytes()).unwrap();
        let from_bytes = bincode::serialize(&self.from).unwrap();
        let from_len = from_bytes.len() as u32;
        len += buf.write(&from_len.to_be_bytes()).unwrap();
        len += buf.write(&from_bytes).unwrap();
        assert_eq!(len, 16 + from_bytes.len());
        len
    }

    fn deserialize(data: &[u8]) -> (Self, usize) {
        let tag = u64::from_be_bytes(data[..8].try_into().unwrap());
        let data_len = u32::from_be_bytes(data[8..12].try_into().unwrap());
        let from_len = u32::from_be_bytes(data[12..16].try_into().unwrap()) as usize;
        let from = bincode::deserialize(&data[16..16 + from_len]).unwrap();
        (MsgHeader::new(tag, data_len, from), 16 + from_len)
    }
}

impl<'a> SendMsg<'a> {
    fn new(tag: u64, data: &'a mut [IoSlice<'a>], from: SocketAddr) -> Self {
        let data_len = data.iter().fold(0, |acc, item| acc + item.len());
        Self {
            header: Some(MsgHeader::new(tag, data_len as _, from)),
            bufs: data,
        }
    }
}

impl RecvMsg {
    pub fn new(ep_id: u32) -> Self {
        RecvMsg {
            ep_id,
            header: MsgHeader::new(0, 0, "127.0.0.1:0".parse().unwrap()),
            data: None,
        }
    }

    #[inline]
    fn take(mut self) -> Bytes {
        self.data.take().unwrap().freeze()
    }
}

impl<'a> transport::SendMsg for SendMsg<'a> {
    #[inline]
    fn pack(&mut self, mut buf: &mut [u8]) -> (usize, bool) {
        let header_len = if let Some(header) = self.header.take() {
            let size = header.serialize(&mut buf);
            buf = &mut buf[size..];
            size
        } else {
            0
        };
        let n = buf.write_vectored(self.bufs).unwrap();
        advance_slices(&mut self.bufs, n);

        (n + header_len, !self.bufs.is_empty())
    }
}

impl transport::RecvMsg for RecvMsg {
    #[inline]
    fn unpack(&mut self, mut pkt: &[u8]) -> bool {
        if self.data.is_none() {
            let (header, len) = MsgHeader::deserialize(pkt);
            self.header = header;
            self.data = Some(BytesMut::with_capacity(self.header.data_len as _));
            pkt = &pkt[len..];
        }
        let buf = unsafe { self.data.as_mut().unwrap_unchecked() };
        buf.put(pkt);

        false
    }
}

impl MsgBuffer {
    fn push(&mut self, mut msg: RecvMsg) {
        let tag = msg.header.tag;
        if let Some(queue) = self.registered.get_mut(&tag) {
            while let Some(sender) = queue.pop_front() {
                msg = match sender.send(msg) {
                    Ok(_) => return,
                    Err(m) => m,
                }
            }
        }
        self.msgs.entry(tag).or_default().push_back(msg);
    }

    fn pop(&mut self, tag: u64) -> oneshot::Receiver<RecvMsg> {
        let (tx, rx) = oneshot::channel();
        if let Some(queue) = self.msgs.get_mut(&tag) {
            if let Some(msg) = queue.pop_front() {
                tx.send(msg).unwrap();
                return rx;
            }
        }
        self.registered.entry(tag).or_default().push_back(tx);
        rx
    }
}

impl transport::Context for Endpoint {
    type SendMsg = SendMsg<'static>;

    type RecvMsg = RecvMsg;

    #[inline]
    fn accept(&mut self, _addr: &str, _ep_id: u32) {
        // todo: notify upper layer
    }

    fn msg_begin(&mut self, ep_id: u32) -> Self::RecvMsg {
        RecvMsg::new(ep_id)
    }

    fn msg_end(&mut self, msg: Self::RecvMsg) {
        self.inner().msg_buf.lock().unwrap().push(msg);
    }
}

impl Inner {
    /// Create a RpcInner bind to `addr` and `dev`
    fn new(addr: SocketAddr, dev: &str) -> io::Result<Self> {
        Ok(Self {
            addr,
            transport: VerbsTransport::new_verbs(dev).map_err(mad_rpc_err_to_io_err)?,
            mappings: AsyncMutex::new(HashMap::new()),
            tasks: Mutex::new(Vec::new()),
            msg_buf: Mutex::new(Default::default()),
        })
    }

    fn url(&self) -> String {
        self.transport.addr()
    }

    async fn send_to_vectored<'a>(
        &self,
        dst: impl ToSocketAddrs,
        tag: u64,
        bufs: &'a mut [IoSlice<'a>],
    ) -> io::Result<()> {
        let dst = lookup_host(dst).await?.next().unwrap();
        let dst_ep_id = self.get_ep_id_or_connect(dst).await?;
        let msg = SendMsg::new(tag, bufs, self.addr);
        //Safety: bufs must live until send msg return
        self.transport
            .send(dst_ep_id, unsafe { std::mem::transmute(msg) })
            .await
            .map_err(mad_rpc_err_to_io_err)?;
        Ok(())
    }

    /// Receives a raw message.
    async fn recv_from_raw(&self, tag: u64) -> io::Result<(Bytes, SocketAddr)> {
        let recver = self.msg_buf.lock().unwrap().pop(tag);
        let msg = recver.await.unwrap();
        let from = msg.header.from;
        let data = msg.take();
        Ok((data, from))
    }

    /// Get the Endpoint Id of the remote rdma peer.
    /// If there has been no connection, try to establish one.
    async fn get_ep_id_or_connect(&self, peer: SocketAddr) -> io::Result<u32> {
        let mut mapping = self.mappings.lock().await;
        if let Some(ep_id) = mapping.get(&peer) {
            return Ok(*ep_id);
        }
        let mut stream = TcpStream::connect(peer).await?;
        let len = stream.read_u32().await? as usize;
        let mut buf = vec![0u8; len];
        let size = stream.read_exact(&mut buf).await?;
        assert_eq!(size, len);
        // url is the address of peer at the level of rdma
        let url = std::str::from_utf8(&buf).expect("Invalid utf-8 bytes receive from peers");
        let ep_id = self
            .transport
            .connect(url)
            .await
            .map_err(mad_rpc_err_to_io_err)?;
        mapping.insert(peer, ep_id);
        Ok(ep_id)
    }
}

impl Endpoint {
    /// Creates a [`Endpoint`] from the given address.
    pub async fn bind(addr: impl ToSocketAddrs) -> io::Result<Self> {
        // This tcp listener is used for helping to establish rdma connection
        let addr = lookup_host(addr).await?.next().unwrap();
        let ep = Endpoint {
            inner: None,
            init_lock: Arc::new(AsyncMutex::new(Some(addr))),
        };
        Ok(ep)
    }

    pub async fn init(mut self, dev: &str) -> io::Result<Self> {
        let listener = {
            let mut guard = self.init_lock.lock().await;
            let addr = guard.take().expect("Duplicate Initialization");
            let listener = TcpListener::bind(addr).await?;
            let addr = listener.local_addr()?;
            self.inner = Some(Arc::new(Inner::new(addr, dev)?));
            listener
        };
        let ep_clone = self.clone();
        //polling
        // todo spwan blocking ?
        let polling_task = task::spawn(async move {
            loop {
                // `ep_clone` accounts for 1 strong count
                // so if strong count of ep is > 1, means Endpoint still in use
                if Arc::strong_count(&ep_clone.inner()) > 1 {
                    ep_clone.inner().transport.progress(&mut ep_clone.clone());
                    task::yield_now().await;
                } else {
                    break;
                }
            }
        });
        let url = self.inner().url();
        // Connection Helper
        let connect_task = task::spawn(async move {
            loop {
                let (mut stream, _) = listener.accept().await.unwrap();
                stream.write_u32(url.as_bytes().len() as _).await.unwrap();
                stream.write_all(url.as_bytes()).await.unwrap();
            }
        });
        self.inner()
            .tasks
            .lock()
            .unwrap()
            .extend([polling_task, connect_task]);
        Ok(self)
    }

    /// Returns the local socket address.
    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        Ok(self.inner().addr)
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
        self.send_to_vectored(dst, tag, &mut [IoSlice::new(data)])
            .await
    }

    /// Like [`send_to`], except that it writes from a slice of buffers.
    ///
    /// [`send_to`]: Endpoint::send_to
    pub async fn send_to_vectored<'a>(
        &self,
        dst: impl ToSocketAddrs,
        tag: u64,
        bufs: &'a mut [IoSlice<'a>],
    ) -> io::Result<()> {
        self.inner().send_to_vectored(dst, tag, bufs).await
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
    pub async fn recv_from_raw(&self, tag: u64) -> io::Result<(Bytes, SocketAddr)> {
        self.inner().recv_from_raw(tag).await
    }

    #[inline]
    fn inner(&self) -> Arc<Inner> {
        self.inner
            .as_ref()
            .expect("Endpoint has not been init")
            .clone()
    }
}

impl Drop for Inner {
    fn drop(&mut self) {
        for task in self.tasks.lock().unwrap().iter() {
            task.abort();
        }
    }
}
