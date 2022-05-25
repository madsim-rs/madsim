use super::rpc::{Bytes, Request};
use crate::task;
use bytes::{BufMut, BytesMut};
use erased_serde::Serialize;
use futures::Future;
use log::warn;
use mad_rpc::{
    transport::{self, Transport},
    ud::VerbsTransport,
};
use std::{
    collections::HashMap,
    io::{self, Write},
    net::SocketAddr,
    sync::{atomic::AtomicU64, Arc, Mutex, RwLock},
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream, ToSocketAddrs},
    sync::{mpsc, oneshot, Mutex as AsyncMutex},
};

// The origin implementation in mad_rpc is not suitable for porting to madsim directly.
// We need modify some details of the rpc by ourselves.
// The main logic is copy from [`mad_rpc::rpc`]

#[derive(Clone)]
pub struct Endpoint {
    /// the addr of local address
    addr: SocketAddr,
    inner: Arc<Inner>,
}

struct Inner {
    transport: Mutex<VerbsTransport<Endpoint>>,
    /// the requester waiting to recv rpc response
    waiters: Mutex<HashMap<u64, oneshot::Sender<RecvMsg>>>,
    /// mappings from SocketAddr to EndpointID in erpc
    mappings: AsyncMutex<HashMap<SocketAddr, u32>>,
    req_buf: RwLock<HashMap<u64, mpsc::UnboundedSender<RecvMsg>>>,
    req_id: AtomicU64,
    tasks: Mutex<Vec<task::JoinHandle<()>>>,
}

#[derive(Default, Debug)]
pub struct RpcHeader {
    rpc_id: u64,   // RPC id
    req_id: u64,   // request or response id
    is_req: bool,  // request or response
    arg_len: u32,  // request arguments len
    data_len: u64, // data len
}

pub struct SendMsg<'a> {
    header: RpcHeader,                       // RPC header
    arg: Option<&'a (dyn Serialize + Sync)>, // RPC arguments
    data: &'a [u8],                          // RPC data
}

#[derive(Debug)]
pub struct RecvMsg {
    ep_id: u32,
    header: RpcHeader,
    data: Option<BytesMut>,
}

fn mad_rpc_err_to_io_err(err: mad_rpc::Error) -> io::Error {
    io::Error::new(io::ErrorKind::Other, format!("{err:?}"))
}

impl<'a> SendMsg<'a> {
    fn new(
        req: bool,
        rpc_id: u64,
        req_id: u64,
        arg: &'a (dyn Serialize + Sync),
        data: Option<&'a [u8]>,
    ) -> Self {
        Self {
            header: RpcHeader {
                is_req: req,
                rpc_id,
                req_id,
                arg_len: 0,
                data_len: 0,
            },
            arg: Some(arg),
            data: data.unwrap_or_default(),
        }
    }
}

impl RecvMsg {
    pub fn new(ep_id: u32) -> Self {
        RecvMsg {
            ep_id,
            header: Default::default(),
            data: None,
        }
    }

    #[inline]
    fn take(mut self) -> (Bytes, Bytes) {
        let buf = self.data.take().unwrap().freeze();
        let arg = buf.slice(0..self.header.arg_len as usize);
        let data = buf.slice(self.header.arg_len as usize..);
        assert_eq!(data.len(), self.header.data_len as usize);

        (arg, data)
    }
}

impl RpcHeader {
    #[inline]
    fn serialize(&self, mut buf: &mut [u8]) {
        let mut len = 0;
        len += buf.write(&self.rpc_id.to_be_bytes()).unwrap();
        len += buf.write(&self.req_id.to_be_bytes()).unwrap();
        len += buf.write(&(self.is_req as u32).to_be_bytes()).unwrap();
        len += buf.write(&self.arg_len.to_be_bytes()).unwrap();
        len += buf.write(&self.data_len.to_be_bytes()).unwrap();

        debug_assert_eq!(len, 32);
    }

    #[inline]
    fn deserialize(buf: &[u8]) -> Self {
        let rpc_id = u64::from_be_bytes(buf[0..8].try_into().unwrap());
        let req_id = u64::from_be_bytes(buf[8..16].try_into().unwrap());
        let is_req = u32::from_be_bytes(buf[16..20].try_into().unwrap()) != 0;
        let arg_len = u32::from_be_bytes(buf[20..24].try_into().unwrap());
        let data_len = u64::from_be_bytes(buf[24..32].try_into().unwrap());

        Self {
            rpc_id,
            req_id,
            is_req,
            arg_len,
            data_len,
        }
    }
}

impl<'a> transport::SendMsg for SendMsg<'a> {
    #[inline]
    fn pack(&mut self, mut buf: &mut [u8]) -> (usize, bool) {
        struct SliceWriter<'a> {
            slice: &'a mut [u8],
            len: usize,
        }

        impl<'a> SliceWriter<'a> {
            pub fn new(buf: &'a mut [u8]) -> Self {
                SliceWriter { slice: buf, len: 0 }
            }

            pub fn len(&'a self) -> usize {
                self.len
            }
        }

        impl Write for SliceWriter<'_> {
            fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
                let len = self.slice.write(buf)?;
                self.len += len;
                Ok(len)
            }

            fn flush(&mut self) -> io::Result<()> {
                Ok(())
            }
        }

        let mut header_len = 0;
        if let Some(arg) = self.arg.take() {
            let mut writer = SliceWriter::new(&mut buf[32..]);
            bincode::serialize_into(&mut writer, arg).unwrap();
            self.header.arg_len = writer.len() as u32;
            self.header.data_len = self.data.len() as u64;
            self.header.serialize(&mut buf[0..32]);
            header_len = self.header.arg_len as usize + 32;
            buf = &mut buf[header_len..];
        }

        let data_len = std::cmp::min(buf.len(), self.data.len());
        buf[0..data_len].copy_from_slice(&self.data[0..data_len]);
        self.data = &self.data[data_len..];

        (header_len + data_len, !self.data.is_empty())
    }
}

impl transport::RecvMsg for RecvMsg {
    #[inline]
    fn unpack(&mut self, mut pkt: &[u8]) -> bool {
        if self.data.is_none() {
            self.header = RpcHeader::deserialize(pkt);
            pkt = &pkt[32..];
            let cap = self.header.arg_len as usize + self.header.data_len as usize;
            let bytes = BytesMut::with_capacity(cap);
            self.data = Some(bytes);
        }

        let buf = unsafe { self.data.as_mut().unwrap_unchecked() };
        buf.put(pkt);

        false
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
        if msg.header.is_req {
            self.inner.on_request(msg)
        } else {
            self.inner.on_response(msg)
        }
    }
}

impl Inner {
    fn new(dev: &str) -> io::Result<Self> {
        Ok(Self {
            transport: VerbsTransport::new_verbs(dev).map_err(mad_rpc_err_to_io_err)?,
            waiters: Mutex::new(HashMap::new()),
            req_id: AtomicU64::new(0),
            req_buf: RwLock::new(HashMap::new()),
            mappings: AsyncMutex::new(HashMap::new()),
            tasks: Mutex::new(Vec::new()),
        })
    }

    fn url(&self) -> String {
        self.transport.addr()
    }

    async fn send_req<'a, R: Request>(
        &self,
        ep_id: u32,
        req_id: u64,
        arg: &R,
        data: Option<&[u8]>,
    ) -> io::Result<()> {
        let rpc_id = R::ID;
        let msg = SendMsg::new(true, rpc_id, req_id, arg, data);
        self.transport
            .send(ep_id, unsafe { std::mem::transmute(msg) })
            .await
            .map_err(mad_rpc_err_to_io_err)?;
        Ok(())
    }

    async fn send_resp<Resp>(
        &self,
        ep_id: u32,
        rpc_id: u64,
        req_id: u64,
        arg: &Resp,
        data: Option<&[u8]>,
    ) -> io::Result<()>
    where
        Resp: Serialize + Sync,
    {
        let msg = SendMsg::new(false, rpc_id, req_id, arg, data);
        self.transport
            .send(ep_id, unsafe { std::mem::transmute(msg) })
            .await
            .map_err(mad_rpc_err_to_io_err)?;
        Ok(())
    }

    fn on_request(&self, msg: RecvMsg) {
        let rpc_id = msg.header.rpc_id;
        self.req_buf
            .read()
            .unwrap()
            .get(&rpc_id)
            .expect("RPC handler not found")
            .send(msg)
            .unwrap();
    }

    fn on_response(&self, msg: RecvMsg) {
        if let Some(value) = self.waiters.lock().unwrap().remove(&msg.header.req_id) {
            value.send(msg).unwrap();
        } else {
            warn!("request {} get response, but no waiter.", msg.header.req_id);
        }
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
    pub async fn bind(addr: impl ToSocketAddrs, dev: &str) -> io::Result<Self> {
        // This tcp listener is used for helping to establish rdma connection
        let listener = TcpListener::bind(addr).await?;
        let addr = listener.local_addr()?;
        let inner = Arc::new(Inner::new(dev)?);
        let ep = Endpoint {
            addr,
            inner: inner.clone(),
        };
        let url = inner.url();
        let ep_clone = ep.clone();
        //polling
        // todo spwan blocking ?
        let polling_task = task::spawn(async move {
            loop {
                // `ep_clone` accounts for 1 strong count
                // so if strong count of ep is > 1, means Endpoint still in use
                if Arc::strong_count(&ep_clone.inner) > 1 {
                    ep_clone.inner.transport.progress(&mut ep_clone.clone());
                    task::yield_now().await;
                } else {
                    break;
                }
            }
        });
        // Connection Helper
        let connect_task = task::spawn(async move {
            loop {
                let (mut stream, _) = listener.accept().await.unwrap();
                stream.write_u32(url.as_bytes().len() as _).await.unwrap();
                stream.write_all(url.as_bytes()).await.unwrap();
            }
        });
        ep.inner
            .tasks
            .lock()
            .unwrap()
            .extend([polling_task, connect_task]);
        Ok(ep)
    }

    /// Returns the local socket address.
    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        Ok(self.addr)
    }
}

impl Endpoint {
    /// Add a RPC handler that send and receive data.
    pub fn add_rpc_handler_with_data<R: Request, AsyncFn, Fut>(self: &Arc<Self>, mut f: AsyncFn)
    where
        AsyncFn: FnMut(R, Bytes) -> Fut + Send + 'static,
        Fut: Future<Output = (R::Response, Vec<u8>)> + Send + 'static,
    {
        let rpc_id = R::ID;
        let (tx, mut rx) = mpsc::unbounded_channel();
        let old_tx = self.inner.req_buf.write().unwrap().insert(rpc_id, tx);
        assert!(
            old_tx.is_none(),
            "RPC ID of {} has multiple handlers",
            rpc_id
        );
        // only use weak pointer to prevent polling task nerver stop
        let inner_weak = Arc::downgrade(&self.inner);
        let rpc_handle_task = task::spawn(async move {
            while let Some(msg) = rx.recv().await {
                assert!(msg.header.is_req && msg.header.rpc_id == rpc_id);

                let ep_id = msg.ep_id;
                let rpc_id = msg.header.rpc_id;
                let req_id = msg.header.req_id;
                let (arg, data) = msg.take();
                let arg = bincode::deserialize(&arg).unwrap();
                let resp_fut = f(arg, data);
                if let Some(inner) = inner_weak.upgrade() {
                    task::spawn(async move {
                        let (resp, resp_data) = resp_fut.await;
                        inner
                            .send_resp(ep_id, rpc_id, req_id, &resp, Some(resp_data.as_ref()))
                            .await
                            .unwrap();
                    });
                }
            }
        });
        self.inner.tasks.lock().unwrap().push(rpc_handle_task);
    }

    pub async fn call_with_data<R: Request>(
        &self,
        dst: SocketAddr,
        request: R,
        data: &[u8],
    ) -> io::Result<(R::Response, Bytes)> {
        struct WaiterGuard<'a>(&'a Inner, u64);
        impl<'a> Drop for WaiterGuard<'a> {
            fn drop(&mut self) {
                self.0.waiters.lock().unwrap().remove(&self.1);
            }
        }

        let ep_id = self.inner.get_ep_id_or_connect(dst).await?;
        // increase req_id
        let req_id = self
            .inner
            .req_id
            .fetch_add(1, std::sync::atomic::Ordering::AcqRel);
        let (tx, rx) = oneshot::channel();
        let guard = WaiterGuard(&self.inner, req_id);
        // register requester for waiting response
        let old_waiter = self.inner.waiters.lock().unwrap().insert(req_id, tx);
        assert!(old_waiter.is_none());
        self.inner
            .send_req(ep_id, req_id, &request, Some(data))
            .await?;
        let msg = rx.await.unwrap();
        std::mem::forget(guard);

        let (resp, data) = msg.take();
        Ok((bincode::deserialize(&resp).unwrap(), data))
    }
}

impl Drop for Inner {
    fn drop(&mut self) {
        for task in self.tasks.lock().unwrap().iter() {
            task.abort();
        }
    }
}
