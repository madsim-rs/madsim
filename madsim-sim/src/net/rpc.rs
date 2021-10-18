//! todo: add documentation
//!
use super::*;

use bytes::{Buf, Bytes};
use core::future::Future;
use core::marker::PhantomData;
use serde::ser::Serialize;
use serde::Deserialize;
use thiserror::Error;

/// RPC type.
pub trait RpcType<'a>: Send {
    /// RPC ID.
    const ID: u64;
    /// Request type.
    type Req: Serialize + Deserialize<'a>;
    /// Response type.
    type Resp: Serialize + Deserialize<'a>;
}

/// Marker: RPC has input data.
pub trait RpcInData {}
/// Marker: RPC has output data.
pub trait RpcOutData {}

/// RPC error.
#[derive(Debug, Error)]
pub enum Error {
    /// Network transport failure.
    #[error("Transport Failed")]
    TransportFailed,
    /// Buffer too small.
    #[error("Buffer Too Small")]
    BufferTooSmall,
}

/// Client side RPC request.
/// RpcRequest can be used many times and can be broadcast to many servers.
pub struct RpcRequest<'a, Rpc: RpcType<'a>> {
    request: Bytes,
    send_data: Option<&'a [u8]>,
    recv_data: Option<&'a mut [u8]>,
    _marker: PhantomData<Rpc>,
}

impl<'a, Rpc: RpcType<'a>> RpcRequest<'a, Rpc> {
    /// New RPC request without input data.
    pub fn new(request: Rpc::Req) -> Self {
        Self {
            request: Bytes::from(bincode::serialize(&request).unwrap()),
            send_data: None,
            recv_data: None,
            _marker: PhantomData,
        }
    }

    /// Net RPC request with input data.
    pub fn send(self, data: &'a [u8]) -> Self
    where
        Rpc: RpcInData,
    {
        Self {
            send_data: Some(data),
            ..self
        }
    }

    /// Provide a buffer to receive data from response.
    /// This function requires RPC interface contains `OutData`.
    ///
    /// Caller should make sure that buffer has enough space to receive response data.
    /// Otherwise, `call()` will return `Err(Error::BufferTooSmall)`.
    pub fn recv(self, data: &'a mut [u8]) -> Self
    where
        Rpc: RpcOutData,
    {
        Self {
            recv_data: Some(data),
            ..self
        }
    }

    /// Set RPC timeout.
    pub fn with_timeout(self, _timeout: std::time::Duration) -> Self {
        // todo: implement timeout
        self
    }

    /// Call RPC on remote server.
    pub async fn call<'n>(
        &mut self,
        network: &'n NetLocalHandle,
        dst: SocketAddr,
    ) -> Result<RpcResponse<Rpc>, Error> {
        network.call(dst, self).await
    }
}

/// Client side RPC response.
pub struct RpcResponse<Rpc> {
    resp: Bytes,
    recv_len: usize,
    _marker: PhantomData<Rpc>,
}

impl<'a, Rpc: RpcType<'a>> RpcResponse<Rpc> {
    fn new(resp: Bytes, recv_len: usize) -> Self {
        Self {
            resp,
            recv_len,
            _marker: PhantomData,
        }
    }

    /// RPC response.
    pub fn response(&'a self) -> Rpc::Resp {
        // decode response here
        bincode::deserialize(&self.resp).unwrap()
    }

    /// The length of received data.
    pub fn recv_len(&self) -> usize {
        self.recv_len
    }
}

/// RPC data, this might a buffer with data received from network.
/// Or just a data descriptor which contains information about how to receive data.
/// This can be used to support rendezvous protocol under RDMA network.
pub enum RpcData {
    /// Data already received from network.
    Data(Bytes),
    /// Data descriptor about how to received data.
    Desc {
        // Currently, sim network doesn't support this.
    },
}

impl RpcData {
    /// Data len.
    pub fn len(&self) -> usize {
        match self {
            Self::Data(bytes) => bytes.len(),
            Self::Desc {} => unreachable!(),
        }
    }

    /// Is data empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Get data.
    pub async fn get(&mut self) -> Result<Bytes, ()> {
        match self {
            Self::Data(bytes) => Ok(bytes.clone()),
            Self::Desc {} => unreachable!(),
        }
    }

    /// Receive RPC data into caller provided buffer.
    /// Buffer should have enough space, otherwise `receive` will return `Err(Error::BufferTooSmall)`.
    /// RpcData will become empty after this.
    pub async fn receive(&mut self, buf: &mut [u8]) -> Result<usize, Error> {
        if buf.len() < self.len() {
            return Err(Error::BufferTooSmall);
        }

        match self {
            Self::Data(bytes) => {
                let len = bytes.len();
                std::mem::replace(bytes, Bytes::new()).copy_to_slice(&mut buf[..len]);
                Ok(len)
            }
            Self::Desc {} => unreachable!(),
        }
    }

    /// Under rendezvous protocol, send operation might block before data is received by peer.
    /// This depends on the implementation of the network protocol.
    /// If server don't receive data, it should call `discard` function to notify peer.
    /// If data has already been received, `discard` will drop underlying bytes.
    /// This function will be called on `Request::reply` and `Request::reply_data`.
    pub async fn discard(&mut self) {
        match self {
            Self::Data(bytes) => bytes.clear(),
            Self::Desc {} => unreachable!(),
        }
    }
}

impl Drop for RpcData {
    fn drop(&mut self) {
        // If rendezvous data is not received before RpcData dropped,
        // we should also notify client.
    }
}

/// Server side RPC request.
pub struct Request<Rpc> {
    network: NetLocalHandle,
    remote: SocketAddr,
    resp_tag: u64,
    request: Bytes,
    data: RpcData,
    _marker: PhantomData<Rpc>,
}

impl<'a, Rpc: RpcType<'a>> Request<Rpc> {
    /// New RPC request
    fn new(
        network: NetLocalHandle,
        remote: SocketAddr,
        resp_tag: u64,
        request: Bytes,
        data: RpcData,
    ) -> Self {
        Self {
            network,
            remote,
            resp_tag,
            request,
            data,
            _marker: PhantomData,
        }
    }

    /// RPC data.
    pub fn data(&mut self) -> &mut RpcData {
        &mut self.data
    }

    /// Remote socket address.
    pub fn remote_addr(&self) -> &SocketAddr {
        &self.remote
    }

    async fn reply_internal(mut self, resp: &Rpc::Resp, data: &[u8]) -> Result<(), Error> {
        self.data.discard().await;
        self.network
            .reply(self.remote, self.resp_tag, resp, data)
            .await
    }

    /// RPC request arguments.
    pub fn request(&'a self) -> Rpc::Req {
        bincode::deserialize(&self.request).unwrap()
    }

    /// Get request and data.
    pub fn request_and_data(&'a mut self) -> (Rpc::Req, &'a mut RpcData)
    where
        Rpc: RpcInData,
    {
        let request = bincode::deserialize(&self.request).unwrap();
        (request, &mut self.data)
    }

    /// Reply RPC.
    pub async fn reply(self, resp: &Rpc::Resp) -> Result<(), Error> {
        self.reply_internal(resp, &[]).await
    }

    /// Reply RPC with data.
    pub async fn reply_data(self, resp: &Rpc::Resp, data: &[u8]) -> Result<(), Error>
    where
        Rpc: RpcOutData,
    {
        self.reply_internal(resp, data).await
    }
}

impl NetLocalHandle {
    /// RPC call.
    pub async fn call<'a, Rpc: RpcType<'a>>(
        &self,
        dst: SocketAddr,
        req: &mut RpcRequest<'a, Rpc>,
    ) -> Result<RpcResponse<Rpc>, Error> {
        let resp_tag = self.handle.rand.with(|rng| rng.gen::<u64>());
        let request = req.request.clone();
        let data = req
            .send_data
            .map(|data| Bytes::copy_from_slice(data))
            .unwrap_or_default();
        self.send_to_raw(dst, Rpc::ID, Box::new((resp_tag, request, data)))
            .await
            .map_err(|_| Error::TransportFailed)?;
        let (rsp, from) = self.recv_from_raw(resp_tag).await.unwrap();
        assert_eq!(from, dst);
        let (rsp, data) = *rsp
            .downcast::<(Vec<u8>, Bytes)>()
            .expect("message type mismatch");
        let len = if let Some(recv_data) = req.recv_data.as_mut() {
            let len = recv_data.len().min(data.len());
            recv_data[..len].copy_from_slice(&data[..len]);
            len
        } else {
            0
        };

        Ok(RpcResponse::new(Bytes::from(rsp), len))
    }

    /// Add a RPC handler.
    pub fn add_rpc_handler<AsyncFn, Fut, Rpc>(&self, f: AsyncFn)
    where
        Rpc: for<'a> RpcType<'a>,
        AsyncFn: FnOnce(Request<Rpc>) -> Fut + Send + Clone + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        let net = self.clone();
        crate::task::spawn(async move {
            loop {
                let (data, from) = net.recv_from_raw(Rpc::ID).await.unwrap();
                let (resp_tag, req, data) = *data
                    .downcast::<(u64, Bytes, Bytes)>()
                    .expect("message type mismatch");
                let net = net.clone();
                let f = f.clone();
                crate::task::spawn(async move {
                    let request = Request::new(net, from, resp_tag, req, RpcData::Data(data));
                    f(request).await;
                })
                .detach();
            }
        })
        .detach();
    }

    /// Send response.
    pub async fn reply<Resp: Serialize>(
        &self,
        dst: SocketAddr,
        tag: u64,
        resp: &Resp,
        data: &[u8],
    ) -> Result<(), Error> {
        let resp = bincode::serialize(resp).unwrap();
        let data = Bytes::copy_from_slice(data);
        self.send_to_raw(dst, tag, Box::new((resp, data)))
            .await
            .map_err(|_| Error::TransportFailed)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::Runtime;
    use std::collections::BTreeMap;
    use std::sync::Mutex;

    #[test]
    fn kv_store() {
        let runtime = Runtime::new();
        let server = runtime.create_host("0.0.0.1:1").unwrap();
        let client = runtime.create_host("0.0.0.2:1").unwrap();
        let server_addr = server.local_addr();

        server
            .spawn(async move {
                let net = NetLocalHandle::current();
                let server = Arc::new(KvServer::new());
                server.register_kv(&net);
            })
            .detach();

        let f = client.spawn(async move {
            let net = NetLocalHandle::current();
            kv_test(&net, server_addr).await;
        });
        runtime.block_on(f);
    }

    #[madsim_macros::service]
    mod kv {
        extern crate self as madsim;

        // Get value of `key`.
        fn get<'a>(key: &'a str, value: &'a mut [u8]) -> bool;

        // Set value of `key`, return old value if key exists.
        fn put<'a>(key: &'a str, new: &'a [u8], old: &'a mut [u8]) -> bool;

        // Get next `key`.
        fn next<'a>(key: &'a str) -> Option<&'a str>;

        // Ping server.
        fn ping<'a>();
    }

    struct KvServer {
        map: Mutex<BTreeMap<String, Bytes>>,
    }

    impl KvServer {
        fn new() -> Self {
            Self {
                map: Mutex::new(BTreeMap::new()),
            }
        }

        fn register_kv(self: Arc<Self>, net: &NetLocalHandle) {
            let server = self.clone();
            net.add_rpc_handler(move |req| server.ping(req));
            let server = self.clone();
            net.add_rpc_handler(move |req| server.get(req));
            let server = self.clone();
            net.add_rpc_handler(move |req| server.put(req));
            let server = self;
            net.add_rpc_handler(move |req| server.next(req));
        }

        async fn ping(self: Arc<Self>, req: Request<kv::Ping>) {
            req.reply(&()).await.unwrap();
        }

        async fn get(self: Arc<Self>, req: Request<kv::Get>) {
            let key = req.request();
            let data = self.map.lock().unwrap().get(key).cloned();
            req.reply_data(&data.is_some(), &data.unwrap_or_default())
                .await
                .unwrap();
        }

        async fn put(self: Arc<Self>, mut req: Request<kv::Put>) {
            let (key, value) = req.request_and_data();
            let value = value.get().await.unwrap();
            let data = self.map.lock().unwrap().insert(key.into(), value);

            req.reply_data(&data.is_some(), &data.unwrap_or_default())
                .await
                .unwrap();
        }

        async fn next(self: Arc<Self>, req: Request<kv::Next>) {
            let key = req.request();
            let next = self
                .map
                .lock()
                .unwrap()
                .range(key.to_owned()..)
                .nth(1)
                .map(|kv| kv.0.to_owned());
            req.reply(&next.as_ref().map(|s| s.as_str())).await.unwrap();
        }
    }

    async fn kv_test(net: &NetLocalHandle, server: SocketAddr) {
        kv::ping().call(net, server).await.unwrap();
        let key = "key";
        let value1 = vec![1_u8; 512];
        let value2 = vec![4_u8; 1024];

        let mut buf = vec![0_u8; 1024];
        let resp = kv::get(&key, &mut buf).call(net, server).await.unwrap();
        assert!(!resp.response());
        assert_eq!(resp.recv_len(), 0);

        let resp = kv::put(&key, &value1, &mut buf)
            .call(net, server)
            .await
            .unwrap();
        assert!(!resp.response());
        assert_eq!(resp.recv_len(), 0);

        let resp = kv::get(&key, &mut buf).call(net, server).await.unwrap();
        assert!(resp.response());
        assert_eq!(&value1, &buf[..resp.recv_len()]);

        let resp = kv::put(&key, &value2, &mut buf)
            .call(net, server)
            .await
            .unwrap();
        assert!(resp.response());
        assert_eq!(&value1, &buf[..resp.recv_len()]);

        let resp = kv::get(&key, &mut buf).call(net, server).await.unwrap();
        assert!(resp.response());
        assert_eq!(&value2, &buf[..resp.recv_len()]);

        let resp = kv::get(&key, &mut buf).call(net, server).await.unwrap();
        assert!(resp.response());
        assert_eq!(&value2, &buf[..resp.recv_len()]);

        let resp = kv::next(&key).call(net, server).await.unwrap();
        let next = resp.response();
        assert!(next.is_none());
    }
}
