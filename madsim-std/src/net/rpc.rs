use super::*;
use futures::FutureExt;
use rand::Rng;
use serde::{de::DeserializeOwned, Serialize};
use std::{
    any::{Any, TypeId},
    fmt::Debug,
    future::Future,
    mem::transmute,
    time::Duration,
};

/// A message that can be sent over the network.
pub trait Message: Debug + Serialize + DeserializeOwned + Any + Send + Sync {}

impl<T: Debug + Serialize + DeserializeOwned + Any + Send + Sync> Message for T {}

impl NetLocalHandle {
    /// Call function on a remote host with timeout.
    pub async fn call_timeout<Req, Rsp>(
        &self,
        dst: SocketAddr,
        request: Req,
        timeout: Duration,
    ) -> io::Result<Rsp>
    where
        Req: Message,
        Rsp: Message,
    {
        crate::time::timeout(timeout, self.call(dst, request))
            .await
            .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "RPC timeout"))?
    }

    /// Call function on a remote host.
    pub async fn call<Req, Rsp>(&self, dst: SocketAddr, request: Req) -> io::Result<Rsp>
    where
        Req: Message,
        Rsp: Message,
    {
        let (rsp, _data) = self.call_with_data(dst, request, &[]).await?;
        Ok(rsp)
    }

    /// Call function on a remote host.
    pub async fn call_with_data<Req, Rsp>(
        &self,
        dst: SocketAddr,
        request: Req,
        data: &[u8],
    ) -> io::Result<(Rsp, Bytes)>
    where
        Req: Message,
        Rsp: Message,
    {
        let req_tag: u64 = unsafe { transmute(TypeId::of::<Req>()) };
        let rsp_tag = rand::thread_rng().gen::<u64>();
        let rsp_tag_buf = rsp_tag.to_be_bytes();
        let req = flexbuffers::to_vec(request).unwrap();
        let req_len_buf = (req.len() as u32).to_be_bytes();
        let iov = [
            IoSlice::new(&rsp_tag_buf[..]),
            IoSlice::new(&req_len_buf[..]),
            IoSlice::new(&req),
            IoSlice::new(data),
        ];
        self.send_to_vectored(dst, req_tag, &iov).await?;

        let (mut data, from) = self.recv_from_raw(rsp_tag).await?;
        assert_eq!(from, dst);
        let rsp_len = data.get_u32() as usize;
        let rsp_bytes = data.split_to(rsp_len);
        let rsp = flexbuffers::from_slice(&rsp_bytes).unwrap();
        Ok((rsp, data))
    }

    /// Add a RPC handler.
    ///
    /// # Example
    ///
    /// ```
    /// use madsim_std::{Runtime, net::NetLocalHandle};
    ///
    /// let runtime = Runtime::new();
    /// let host1 = runtime.create_host("127.0.0.1:0").unwrap();
    /// let host2 = runtime.create_host("127.0.0.1:0").unwrap();
    /// let addr1 = host1.local_addr();
    /// let addr2 = host2.local_addr();
    ///
    /// host1
    ///     .spawn(async move {
    ///         let net = NetLocalHandle::current();
    ///         net.add_rpc_handler(|x: u64| async move { x + 1 });
    ///         net.add_rpc_handler(|x: u32| async move { x + 2 });
    ///     })
    ///     .detach();
    ///
    /// let f = host2.spawn(async move {
    ///     let net = NetLocalHandle::current();
    ///
    ///     let rsp: u64 = net.call(addr1, 1u64).await.unwrap();
    ///     assert_eq!(rsp, 2u64);
    ///
    ///     let rsp: u32 = net.call(addr1, 1u32).await.unwrap();
    ///     assert_eq!(rsp, 3u32);
    /// });
    ///
    /// runtime.block_on(f);
    /// ```
    pub fn add_rpc_handler<Req, Rsp, AsyncFn, Fut>(&self, mut f: AsyncFn)
    where
        Req: Message,
        Rsp: Message,
        AsyncFn: FnMut(Req) -> Fut + Send + 'static,
        Fut: Future<Output = Rsp> + Send + 'static,
    {
        self.add_rpc_handler_data(move |req, _data| f(req).map(|rsp| (rsp, vec![])))
    }

    /// Add a RPC handler that send and receive data.
    ///
    /// # Example
    ///
    /// ```
    /// use madsim_std::{Runtime, net::{Bytes, NetLocalHandle}};
    ///
    /// let runtime = Runtime::new();
    /// let host1 = runtime.create_host("127.0.0.1:0").unwrap();
    /// let host2 = runtime.create_host("127.0.0.1:0").unwrap();
    /// let addr1 = host1.local_addr();
    /// let addr2 = host2.local_addr();
    ///
    /// host1
    ///     .spawn(async move {
    ///         let net = NetLocalHandle::current();
    ///         net.add_rpc_handler_data(|x: u64, data: Bytes| async move {
    ///             (x + 1, data.as_ref().into())
    ///         });
    ///         net.add_rpc_handler_data(|x: u32, data: Bytes| async move {
    ///             (x + 2, b"hello".to_vec())
    ///         });
    ///     })
    ///     .detach();
    ///
    /// let f = host2.spawn(async move {
    ///     let net = NetLocalHandle::current();
    ///
    ///     let (rsp, data): (u64, _) = net.call_with_data(addr1, 1u64, b"hi").await.unwrap();
    ///     assert_eq!(rsp, 2u64);
    ///     assert_eq!(data, &b"hi"[..]);
    ///
    ///     let (rsp, data): (u32, _) = net.call_with_data(addr1, 1u32, b"hi").await.unwrap();
    ///     assert_eq!(rsp, 3u32);
    ///     assert_eq!(data, &b"hello"[..]);
    /// });
    ///
    /// runtime.block_on(f);
    /// ```
    pub fn add_rpc_handler_data<Req, Rsp, AsyncFn, Fut>(&self, mut f: AsyncFn)
    where
        Req: Message,
        Rsp: Message,
        AsyncFn: FnMut(Req, Bytes) -> Fut + Send + 'static,
        Fut: Future<Output = (Rsp, Vec<u8>)> + Send + 'static,
    {
        let req_tag: u64 = unsafe { transmute(TypeId::of::<Req>()) };
        let net = self.clone();
        crate::task::spawn(async move {
            loop {
                let (mut data, from) = net.recv_from_raw(req_tag).await.unwrap();
                let rsp_tag = data.get_u64();
                let req_len = data.get_u32() as usize;
                let req_bytes = data.split_to(req_len);
                let req: Req = flexbuffers::from_slice(&req_bytes).unwrap();
                let rsp_future = f(req, data);
                let net = net.clone();
                crate::task::spawn(async move {
                    let (rsp, data) = rsp_future.await;
                    let rsp = flexbuffers::to_vec(rsp).unwrap();
                    let rsp_len_buf = (rsp.len() as u32).to_be_bytes();
                    let iov = [
                        IoSlice::new(&rsp_len_buf[..]),
                        IoSlice::new(&rsp),
                        IoSlice::new(&data),
                    ];
                    net.send_to_vectored(from, rsp_tag, &iov).await.unwrap();
                })
                .detach();
            }
        })
        .detach();
    }
}
