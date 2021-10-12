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

/// Rpc error.
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
pub struct RpcRequest<'a, Resp, const ID: u64> {
    // todo: it's better to use &Req here, but it's hard to handle lifetime.
    request: Bytes,
    send_data: Option<&'a [u8]>,
    _marker: PhantomData<Resp>,
}

/// Client side Rpc request with recv buffer.
/// RpcRecvRequest can only be send to one server.
pub struct RpcRecvRequest<'a, Resp, const ID: u64> {
    request: RpcRequest<'a, Resp, ID>,
    recv_data: &'a mut [u8],
}

impl<'a, 'b, Resp: Deserialize<'b>, const ID: u64> RpcRequest<'a, Resp, ID> {
    /// New Rpc request
    pub fn new<Req: Serialize>(request: &Req) -> Self {
        Self {
            request: Bytes::from(bincode::serialize(request).unwrap()),
            send_data: None,
            _marker: PhantomData::default(),
        }
    }

    /// Set Rpc timeout
    pub fn with_timeout(self, _timeout: std::time::Duration) -> Self {
        // todo: implement timeout
        self
    }

    /// Send data with Rpc.
    pub fn send(mut self, data: &'a [u8]) -> Self {
        self.send_data = Some(data);
        self
    }

    /// Provide a buffer to receive data from response.
    /// Caller should make sure that buffer has enough space to receive response data.
    /// Otherwise, `call()` will return `Err(Error::BufferTooSmall)`.
    /// Caller will get empty RpcData from RpcResponse.
    pub fn recv(self, data: &'a mut [u8]) -> RpcRecvRequest<'a, Resp, ID> {
        RpcRecvRequest {
            request: self,
            recv_data: data,
        }
    }

    /// Call Rpc on remote server.
    pub async fn call<'n>(
        &self,
        network: &'n NetLocalHandle,
        dst: SocketAddr,
    ) -> Result<RpcResponse<Resp>, Error> {
        network.call(dst, self, None).await.map(|resp| resp.0)
    }

    /// Broadcast Rpc to all targets.
    pub async fn broadcast(&self, _net: &NetLocalHandle) {
        todo!()
    }
}

impl<'a, 'b, Resp: Deserialize<'b>, const ID: u64> RpcRecvRequest<'a, Resp, ID> {
    /// Call Rpc on remote server.
    pub async fn call<'n>(
        self,
        network: &'n NetLocalHandle,
        dst: SocketAddr,
    ) -> Result<(RpcResponse<Resp>, usize), Error> {
        network.call(dst, &self.request, Some(self.recv_data)).await
    }
}

/// Client side Rpc response
pub struct RpcResponse<Resp> {
    resp: Bytes,
    data: RpcData,
    _marker: PhantomData<Resp>,
}

impl<'a, Resp: Deserialize<'a>> RpcResponse<Resp> {
    fn new(resp: Bytes, data: RpcData) -> Self {
        Self {
            resp,
            data,
            _marker: PhantomData::default(),
        }
    }

    /// Rpc response.
    pub fn response(&'a self) -> Resp {
        // decode response here
        bincode::deserialize(&self.resp).unwrap()
    }

    /// Response data.
    pub fn data(&mut self) -> &mut RpcData {
        &mut self.data
    }
}

/// Rpc data, this might a buffer with data received from network.
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

    /// Receive Rpc data into caller provided buffer.
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

/// Server side Rpc request.
pub struct Request<Req, Resp, const ID: u64> {
    network: NetLocalHandle,
    remote: SocketAddr,
    resp_tag: u64,
    request: Bytes,
    data: RpcData,
    // make request Send.
    _marker: PhantomData<fn() -> (Req, Resp)>,
}

impl<'de, Req: Deserialize<'de>, Resp: Serialize, const ID: u64> Request<Req, Resp, ID> {
    /// New Rpc request
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

    /// Rpc request arguments.
    pub fn request(&'de self) -> Req {
        bincode::deserialize(&self.request).unwrap()
    }

    /// Rpc data.
    pub fn data(&mut self) -> &mut RpcData {
        &mut self.data
    }

    /// Client socket address.
    pub fn remote_addr(&self) -> &SocketAddr {
        &self.remote
    }

    /// Reply Rpc.
    pub async fn reply(self, resp: &Resp) -> Result<(), Error> {
        self.reply_data(resp, &[]).await
    }

    /// Reply Rpc with data.
    pub async fn reply_data(mut self, resp: &Resp, data: &[u8]) -> Result<(), Error> {
        self.data.discard().await;
        self.network
            .reply(self.remote, self.resp_tag, resp, data)
            .await
    }
}

impl NetLocalHandle {
    /// Rpc call.
    pub async fn call<'a, Resp: Deserialize<'a>, const ID: u64>(
        &self,
        dst: SocketAddr,
        req: &RpcRequest<'_, Resp, ID>,
        recv_data: Option<&mut [u8]>,
    ) -> Result<(RpcResponse<Resp>, usize), Error> {
        // It's hard to handle lifetime here, we have to serialize data.
        let resp_tag = self.handle.rand.with(|rng| rng.gen::<u64>());
        let request = req.request.clone();
        let data = req
            .send_data
            .map(|data| Bytes::copy_from_slice(data))
            .unwrap_or_default();
        self.send_to_raw(dst, ID, Box::new((resp_tag, request, data)))
            .await
            .map_err(|_| Error::TransportFailed)?;
        let (rsp, from) = self.recv_from_raw(resp_tag).await.unwrap();
        assert_eq!(from, dst);
        let (rsp, data) = *rsp
            .downcast::<(Vec<u8>, Bytes)>()
            .expect("message type mismatch");

        // todo: avoid extra copy
        let mut data = RpcData::Data(data);
        let data_len = data.len();
        if let Some(recv_data) = recv_data {
            data.receive(recv_data).await?;
            Ok((RpcResponse::new(Bytes::from(rsp), data), data_len))
        } else {
            Ok((RpcResponse::new(Bytes::from(rsp), data), 0))
        }
    }

    /// Add a Rpc handler.
    pub fn add_rpc_handler<'a, Req: Deserialize<'a>, Resp: Serialize, AsyncFn, Fut, const ID: u64>(
        &self,
        f: AsyncFn,
    ) where
        AsyncFn: FnOnce(Request<Req, Resp, ID>) -> Fut + Send + Clone + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        let net = self.clone();
        crate::task::spawn(async move {
            loop {
                let (data, from) = net.recv_from_raw(ID).await.unwrap();
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

    // `#[madsim_macros::service]` will generate code like:
    // mod kv {
    //     use super::*;
    //     pub const GET: u64 = 0;
    //     pub const PUT: u64 = 1;
    //     pub const PING: u64 = 2;
    //
    //     pub fn get<'a>(key: & &str) -> RpcRequest<'a, bool, GET> {
    //         RpcRequest::new(key)
    //     }
    //
    //     pub fn put<'a>(key: & &str) -> RpcRequest<'a, bool, PUT> {
    //         RpcRequest::new(key)
    //     }
    //
    //     pub fn ping<'a>() -> RpcRequest<'a, (), PING> {
    //         RpcRequest::new(&())
    //     }
    // }
    #[madsim_macros::service]
    mod kv {
        // get value of `key`, output data is value
        fn get(key: &str) -> bool;

        // set value of `key`, input data is new value, output data is old value
        fn put(key: &str) -> bool;

        // ping server, no arguments.
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
            let server = self.clone();
            net.add_rpc_handler(move |req| server.put(req));
        }

        async fn ping(self: Arc<Self>, req: Request<(), (), { kv::PING }>) {
            req.reply(&()).await.unwrap()
        }

        async fn get(self: Arc<Self>, req: Request<&str, bool, { kv::GET }>) {
            let data = self.map.lock().unwrap().get(req.request()).cloned();
            req.reply_data(&data.is_some(), &data.unwrap_or(Bytes::new()))
                .await
                .unwrap();
        }

        async fn put(self: Arc<Self>, mut req: Request<&str, bool, { kv::PUT }>) {
            let key = req.request().to_owned();
            let value = req.data().get().await.unwrap();
            let data = self.map.lock().unwrap().insert(key, value);
            req.reply_data(&data.is_some(), &data.unwrap_or(Bytes::new()))
                .await
                .unwrap();
        }
    }

    async fn kv_test(net: &NetLocalHandle, server: SocketAddr) {
        kv::ping().call(net, server).await.unwrap();
        let key = "key";
        let value1 = vec![1; 512];
        let value2 = vec![4; 1024];

        let mut buf = vec![0; 1024];
        let (resp, data_len) = kv::get(&key)
            .recv(&mut buf)
            .call(net, server)
            .await
            .unwrap();
        assert_eq!(resp.response(), false);
        assert_eq!(data_len, 0);

        let resp = kv::put(&key).send(&value1).call(net, server).await.unwrap();
        assert_eq!(resp.response(), false);

        let (resp, data_len) = kv::get(&key)
            .recv(&mut buf)
            .call(net, server)
            .await
            .unwrap();
        assert_eq!(resp.response(), true);
        assert_eq!(&value1, &buf[..data_len]);

        let mut resp = kv::get(&key).call(net, server).await.unwrap();
        assert_eq!(resp.response(), true);
        assert_eq!(resp.data().get().await.unwrap(), &value1);

        let (resp, data_len) = kv::put(&key)
            .send(&value2)
            .recv(&mut buf)
            .call(net, server)
            .await
            .unwrap();
        assert_eq!(resp.response(), true);
        assert_eq!(&value1, &buf[..data_len]);

        let (resp, data_len) = kv::get(&key)
            .recv(&mut buf)
            .call(net, server)
            .await
            .unwrap();
        assert_eq!(resp.response(), true);
        assert_eq!(&value2, &buf[..data_len]);
    }
}
