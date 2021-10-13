//! todo: add documentation
//!
use super::*;

use bincode;
use bytes::{Buf, Bytes};
use core::future::Future;
use core::marker::PhantomData;
use serde::ser::Serialize;
use serde::Deserialize;
use thiserror::Error;

/// RPC type.
pub trait RpcType<Req, Resp> {
    /// RPC ID.
    const ID: u64;
}

/// RPC has input data or not.
pub trait RpcInData<const D: bool> {}
/// RPC has output data or not.
pub trait RpcOutData<const D: bool> {}

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
pub struct RpcRequest<'a, Resp, Rpc> {
    // todo: it's better to use &Req here, but it's hard to handle lifetime with macro,
    //       need a way to validate request type is right.
    request: Bytes,
    send_data: Option<&'a [u8]>,
    _marker: PhantomData<fn() -> (Resp, Rpc)>,
}

/// Client side RPC request with recv buffer.
/// RpcRecvRequest can only be send to one server.
pub struct RpcRecvRequest<'a, Resp, Rpc> {
    request: RpcRequest<'a, Resp, Rpc>,
    recv_data: &'a mut [u8],
}

impl<'a, 'de, Resp: Deserialize<'de>, Rpc> RpcRequest<'a, Resp, Rpc>
where
    Rpc: RpcType<(), Resp> + RpcInData<false>,
{
    /// New RPC request without input data.
    pub fn new<Req: Serialize>(request: &Req) -> Self {
        Self {
            request: Bytes::from(bincode::serialize(request).unwrap()),
            send_data: None,
            _marker: PhantomData::default(),
        }
    }
}

impl<'a, 'de, Resp: Deserialize<'de>, Rpc> RpcRequest<'a, Resp, Rpc>
where
    Rpc: RpcType<(), Resp> + RpcInData<true>,
{
    /// Net RPC request with input data.
    pub fn new_data<Req: Serialize>(request: &Req, data: &'a [u8]) -> Self {
        Self {
            request: Bytes::from(bincode::serialize(request).unwrap()),
            send_data: Some(data),
            _marker: PhantomData::default(),
        }
    }
}

impl<'a, 'de, Resp: Deserialize<'de>, Rpc> RpcRequest<'a, Resp, Rpc>
where
    Rpc: RpcType<(), Resp> + RpcOutData<true>,
{
    /// Provide a buffer to receive data from response.
    /// Caller should make sure that buffer has enough space to receive response data.
    /// Otherwise, `call()` will return `Err(Error::BufferTooSmall)`.
    /// Caller will get empty RpcData from RpcResponse.
    ///
    /// This function requires RPC interface contains `OutData`.
    pub fn recv(self, data: &'a mut [u8]) -> RpcRecvRequest<'a, Resp, Rpc> {
        RpcRecvRequest {
            request: self,
            recv_data: data,
        }
    }
}

impl<'a, 'de, Resp: Deserialize<'de>, Rpc> RpcRequest<'a, Resp, Rpc>
where
    Rpc: RpcType<(), Resp>,
{
    /// Set RPC timeout
    pub fn with_timeout(self, _timeout: std::time::Duration) -> Self {
        // todo: implement timeout
        self
    }

    /// Call RPC on remote server.
    pub async fn call<'n>(
        &self,
        network: &'n NetLocalHandle,
        dst: SocketAddr,
    ) -> Result<RpcResponse<Resp, Rpc>, Error> {
        network.call(dst, self).await
    }

    /// Broadcast RPC to all targets.
    pub async fn broadcast(&self, _net: &NetLocalHandle) {
        todo!()
    }
}

impl<'a, 'de, Resp: Deserialize<'de>, Rpc> RpcRecvRequest<'a, Resp, Rpc>
where
    Rpc: RpcType<(), Resp> + RpcOutData<true>,
{
    /// Call RPC on remote server.
    pub async fn call<'n>(
        mut self,
        network: &'n NetLocalHandle,
        dst: SocketAddr,
    ) -> Result<(RpcRecvResponse<Resp, Rpc>, usize), Error> {
        network.call_with_recv(dst, &mut self).await
    }
}

/// Client side RPC response.
pub struct RpcResponse<Resp, Rpc> {
    resp: Bytes,
    data: RpcData,
    _marker: PhantomData<fn() -> (Resp, Rpc)>,
}

impl<'de, Resp, Rpc> RpcResponse<Resp, Rpc>
where
    Rpc: RpcType<(), Resp>,
{
    fn new(resp: Bytes, data: RpcData) -> Self {
        Self {
            resp,
            data,
            _marker: PhantomData::default(),
        }
    }
}

impl<'de, Resp: Deserialize<'de>, Rpc> RpcResponse<Resp, Rpc>
where
    Rpc: RpcType<(), Resp> + RpcOutData<false>,
{
    /// RPC response.
    pub fn response(&'de self) -> Resp {
        // decode response here
        bincode::deserialize(&self.resp).unwrap()
    }
}

impl<'de, Resp: Deserialize<'de>, Rpc> RpcResponse<Resp, Rpc>
where
    Rpc: RpcType<(), Resp> + RpcOutData<true>,
{
    /// Response and data.
    pub fn response_and_data(&'de mut self) -> (Resp, &'de mut RpcData) {
        (bincode::deserialize(&self.resp).unwrap(), &mut self.data)
    }
}

/// Client side RPC response with data already received.
pub struct RpcRecvResponse<Resp, Rpc> {
    resp: Bytes,
    _marker: PhantomData<fn() -> (Resp, Rpc)>,
}

impl<'de, Resp: Deserialize<'de>, Rpc> RpcRecvResponse<Resp, Rpc>
where
    Rpc: RpcType<(), Resp> + RpcOutData<true>,
{
    fn new(resp: Bytes) -> Self {
        Self {
            resp,
            _marker: PhantomData::default(),
        }
    }

    /// RPC response.
    pub fn response(&'de self) -> Resp {
        // decode response here
        bincode::deserialize(&self.resp).unwrap()
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
pub struct Request<Req, Resp, Rpc> {
    network: NetLocalHandle,
    remote: SocketAddr,
    resp_tag: u64,
    request: Bytes,
    data: RpcData,
    // make request Send.
    #[allow(clippy::type_complexity)]
    _marker: PhantomData<fn() -> (Req, Resp, Rpc)>,
}

impl<Req, Resp: Serialize, Rpc> Request<Req, Resp, Rpc>
where
    Rpc: RpcType<Req, Resp>,
{
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
            _marker: PhantomData::default(),
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

    async fn reply_internal(mut self, resp: &Resp, data: &[u8]) -> Result<(), Error> {
        self.data.discard().await;
        self.network
            .reply(self.remote, self.resp_tag, resp, data)
            .await
    }
}

impl<'de, Req: Deserialize<'de>, Resp: Serialize, Rpc> Request<Req, Resp, Rpc>
where
    Rpc: RpcType<Req, Resp> + RpcInData<false>,
{
    /// RPC request arguments.
    pub fn request(&'de self) -> Req {
        bincode::deserialize(&self.request).unwrap()
    }
}

impl<'de, Req: Deserialize<'de>, Resp: Serialize, Rpc> Request<Req, Resp, Rpc>
where
    Rpc: RpcType<Req, Resp> + RpcInData<true>,
{
    /// Request and data
    pub fn request_and_data<'a>(&'a mut self) -> (Req, &'a mut RpcData)
    where
        Req: 'a,
    {
        (
            bincode::deserialize::<'de, Req>(unsafe {
                std::mem::transmute::<&'a [u8], &'de [u8]>(&self.request)
            })
            .unwrap(),
            &mut self.data,
        )
    }
}

impl<'de, Req: Deserialize<'de>, Resp: Serialize, Rpc> Request<Req, Resp, Rpc>
where
    Rpc: RpcType<Req, Resp> + RpcOutData<false>,
{
    /// Reply RPC without data.
    pub async fn reply(self, resp: &Resp) -> Result<(), Error> {
        self.reply_internal(resp, &[]).await
    }
}

impl<Req, Resp: Serialize, Rpc> Request<Req, Resp, Rpc>
where
    Rpc: RpcType<Req, Resp> + RpcOutData<true>,
{
    /// Reply RPC with data.
    pub async fn reply_data(self, resp: &Resp, data: &[u8]) -> Result<(), Error> {
        self.reply_internal(resp, data).await
    }
}

impl NetLocalHandle {
    /// RPC call.
    pub async fn call<'de, Resp: Deserialize<'de>, Rpc>(
        &self,
        dst: SocketAddr,
        req: &RpcRequest<'_, Resp, Rpc>,
    ) -> Result<RpcResponse<Resp, Rpc>, Error>
    where
        Rpc: RpcType<(), Resp>,
    {
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

        Ok(RpcResponse::new(Bytes::from(rsp), RpcData::Data(data)))
    }

    /// Call RPC with receive buffer.
    pub async fn call_with_recv<'de, Resp: Deserialize<'de>, Rpc>(
        &self,
        dst: SocketAddr,
        req: &mut RpcRecvRequest<'_, Resp, Rpc>,
    ) -> Result<(RpcRecvResponse<Resp, Rpc>, usize), Error>
    where
        Rpc: RpcType<(), Resp> + RpcOutData<true>,
    {
        // todo: avoid extra copy
        let RpcResponse { resp, mut data, .. } = self.call(dst, &req.request).await?;

        // todo: avoid extra copy
        let data_len = data.len();
        data.receive(req.recv_data).await?;
        Ok((RpcRecvResponse::new(resp), data_len))
    }

    /// Add a RPC handler.
    pub fn add_rpc_handler<'de, Req: Deserialize<'de>, Resp: Serialize, AsyncFn, Fut, Rpc>(
        &self,
        f: AsyncFn,
    ) where
        Rpc: RpcType<Req, Resp>,
        AsyncFn: FnOnce(Request<Req, Resp, Rpc>) -> Fut + Send + Clone + 'static,
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
                    // wtf: why can't f(request).await?
                    let fut = f(request);
                    fut.await;
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
    use std::collections::HashMap;
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

    // This macro will generate code for RPC information and client.
    #[madsim_macros::service]
    mod kv {
        // Get value of `key`.
        fn get(key: &str, value: RpcOutData) -> bool;

        // Set value of `key`, retrun old value if key exists.
        fn put(key: &str, new: RpcInData, old: RpcOutData) -> bool;

        // Ping server.
        fn ping();
    }

    struct KvServer {
        map: Mutex<HashMap<String, Bytes>>,
    }

    impl KvServer {
        fn new() -> Self {
            Self {
                map: Mutex::new(HashMap::new()),
            }
        }

        fn register_kv(self: Arc<Self>, net: &NetLocalHandle) {
            let server = self.clone();
            net.add_rpc_handler(move |req| server.ping(req));
            let server = self.clone();
            net.add_rpc_handler(move |req| server.get(req));
            let server = self;
            net.add_rpc_handler(move |req| server.put(req));
        }

        async fn ping(self: Arc<Self>, req: Request<(), (), kv::Ping>) {
            req.reply(&()).await.unwrap()
        }

        async fn get(self: Arc<Self>, req: Request<&str, bool, kv::Get>) {
            let key = req.request();
            let data = self.map.lock().unwrap().get(key).cloned();
            req.reply_data(&data.is_some(), &data.unwrap_or_default())
                .await
                .unwrap();
        }

        async fn put(self: Arc<Self>, mut req: Request<&str, bool, kv::Put>) {
            let (key, value) = req.request_and_data();
            let key = key.to_owned();
            let value = value.get().await.unwrap();
            let data = self.map.lock().unwrap().insert(key, value);

            req.reply_data(&data.is_some(), &data.unwrap_or_default())
                .await
                .unwrap();
        }
    }

    async fn kv_test(net: &NetLocalHandle, server: SocketAddr) {
        kv::ping().call(net, server).await.unwrap();
        let key = "key";
        let value1 = vec![1_u8; 512];
        let value2 = vec![4_u8; 1024];

        let mut buf = vec![0_u8; 1024];
        let (resp, data_len) = kv::get(&key)
            .recv(&mut buf)
            .call(net, server)
            .await
            .unwrap();
        assert!(!resp.response());
        assert_eq!(data_len, 0);

        let mut resp = kv::put(&key, &value1).call(net, server).await.unwrap();
        let (find_old, data) = resp.response_and_data();
        assert!(!find_old);
        assert!(data.is_empty());

        let (resp, data_len) = kv::get(&key)
            .recv(&mut buf)
            .call(net, server)
            .await
            .unwrap();
        assert!(resp.response());
        assert_eq!(&value1, &buf[..data_len]);

        let mut resp = kv::get(&key).call(net, server).await.unwrap();
        let (find, value) = resp.response_and_data();
        assert!(find);
        assert_eq!(&value1, &value.get().await.unwrap());

        let (resp, data_len) = kv::put(&key, &value2)
            .recv(&mut buf)
            .call(net, server)
            .await
            .unwrap();
        assert!(resp.response());
        assert_eq!(&value1, &buf[..data_len]);

        let (resp, data_len) = kv::get(&key)
            .recv(&mut buf)
            .call(net, server)
            .await
            .unwrap();
        assert!(resp.response());
        assert_eq!(&value2, &buf[..data_len]);

        let mut resp = kv::get(&key).call(net, server).await.unwrap();
        let (find, value) = resp.response_and_data();
        assert!(find);
        assert_eq!(&value2, &value.get().await.unwrap());
    }
}
