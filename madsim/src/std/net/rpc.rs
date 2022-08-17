//! RPC support.
//!
//! # Methods
//!
//! RPC extension adds the following methods for [`Endpoint`]:
//!
//! - [`call`][Endpoint::call]
//! - [`call_with_data`][Endpoint::call_with_data]
//! - [`call_timeout`][Endpoint::call_timeout]
//! - [`add_rpc_handler`][Endpoint::add_rpc_handler]
//! - [`add_rpc_handler_with_data`][Endpoint::add_rpc_handler_with_data]
//!
//! # Examples
//!
//! ```
//! use madsim::net::{Endpoint, rpc::*};
//! use std::sync::Arc;
//! use std::net::SocketAddr;
//!
//! #[derive(Serialize, Deserialize)]
//! struct Req1(u32);
//! impl Request for Req1 {
//!     type Response = u32;
//!     const ID: u64 = 1;
//! }
//!
//! #[derive(Serialize, Deserialize)]
//! struct Req2(u32);
//! impl Request for Req2 {
//!     type Response = u32;
//!     const ID: u64 = 2;
//! }
//!
//! let runtime = tokio::runtime::Runtime::new().unwrap();
//! let rpc = runtime.spawn(async move {
//!     let net = Arc::new(Endpoint::bind("127.0.0.1:0").await.unwrap());
//!     net.add_rpc_handler(|x: Req1| async move { x.0 + 1 });
//!     net.add_rpc_handler_with_data(|x: Req2, data| async move {
//!         (x.0 + 2, b"hello".to_vec())
//!     });
//!     net.local_addr().unwrap()
//! });
//!
//! runtime.block_on(async move {
//!     let addr = rpc.await.unwrap();
//!     let net = Endpoint::bind("127.0.0.1:0").await.unwrap();
//!
//!     let rsp = net.call(addr, Req1(1)).await.unwrap();
//!     assert_eq!(rsp, 2);
//!
//!     let (rsp, data) = net.call_with_data(addr, Req2(1), b"hi").await.unwrap();
//!     assert_eq!(rsp, 3);
//!     assert_eq!(data, &b"hello"[..]);
//! });
//! ```

use super::*;
use bytes::Buf;
#[doc(no_inline)]
pub use bytes::Bytes;
use futures_util::FutureExt;
use rand::Rng;
#[doc(no_inline)]
pub use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{
    any::Any,
    future::Future,
    io::{self, IoSlice},
    net::SocketAddr,
    sync::Arc,
    time::Duration,
};

/// A RPC request.
pub trait Request: Serialize + DeserializeOwned + Any + Send + Sync {
    /// A RPC response.
    type Response: Serialize + DeserializeOwned + Any + Send + Sync;

    /// A unique ID.
    const ID: u64;
}

#[doc(hidden)]
pub const fn hash_str(s: &str) -> u64 {
    // simple hash33
    let mut h = 0u64;
    let s = s.as_bytes();
    let mut i = 0;
    while i < s.len() {
        h = h.wrapping_mul(33).wrapping_add(s[i] as u64);
        i += 1;
    }
    h
}

impl Endpoint {
    /// Call function on a remote node with timeout.
    pub async fn call_timeout<R: Request>(
        &self,
        dst: SocketAddr,
        request: R,
        timeout: Duration,
    ) -> io::Result<R::Response> {
        crate::time::timeout(timeout, self.call(dst, request))
            .await
            .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "RPC timeout"))?
    }

    /// Call function on a remote node.
    pub async fn call<R: Request>(&self, dst: SocketAddr, request: R) -> io::Result<R::Response> {
        let (rsp, _data) = self.call_with_data(dst, request, &[]).await?;
        Ok(rsp)
    }

    /// Call function on a remote node.
    pub async fn call_with_data<R: Request>(
        &self,
        dst: SocketAddr,
        request: R,
        data: &[u8],
    ) -> io::Result<(R::Response, Bytes)> {
        let req_tag = R::ID;
        let rsp_tag = rand::thread_rng().gen::<u64>();
        let rsp_tag_buf = rsp_tag.to_be_bytes();
        let req = bincode::serialize(&request).unwrap();
        let req_len_buf = (req.len() as u32).to_be_bytes();
        let mut iov = [
            IoSlice::new(&rsp_tag_buf[..]),
            IoSlice::new(&req_len_buf[..]),
            IoSlice::new(&req),
            IoSlice::new(data),
        ];
        self.send_to_vectored(dst, req_tag, &mut iov).await?;

        let (mut data, from) = self.recv_from_raw(rsp_tag).await?;
        assert_eq!(from, dst);
        let rsp_len = data.get_u32() as usize;
        let rsp_bytes = data.split_to(rsp_len);
        let rsp = bincode::deserialize(&rsp_bytes).unwrap();
        Ok((rsp, data))
    }

    /// Add a RPC handler.
    pub fn add_rpc_handler<R: Request, AsyncFn, Fut>(self: &Arc<Self>, mut f: AsyncFn)
    where
        AsyncFn: FnMut(R) -> Fut + Send + 'static,
        Fut: Future<Output = R::Response> + Send + 'static,
    {
        self.add_rpc_handler_with_data(move |req, _data| f(req).map(|rsp| (rsp, vec![])))
    }

    /// Add a RPC handler that send and receive data.
    pub fn add_rpc_handler_with_data<R: Request, AsyncFn, Fut>(self: &Arc<Self>, mut f: AsyncFn)
    where
        AsyncFn: FnMut(R, Bytes) -> Fut + Send + 'static,
        Fut: Future<Output = (R::Response, Vec<u8>)> + Send + 'static,
    {
        let req_tag = R::ID;
        let net = self.clone();
        crate::task::spawn(async move {
            loop {
                let (mut data, from) = net.recv_from_raw(req_tag).await.unwrap();
                let rsp_tag = data.get_u64();
                let req_len = data.get_u32() as usize;
                let req_bytes = data.split_to(req_len);
                let req: R = bincode::deserialize(&req_bytes).unwrap();
                let rsp_future = f(req, data);
                let net = net.clone();
                crate::task::spawn(async move {
                    let (rsp, data) = rsp_future.await;
                    let rsp = bincode::serialize(&rsp).unwrap();
                    let rsp_len_buf = (rsp.len() as u32).to_be_bytes();
                    let mut iov = [
                        IoSlice::new(&rsp_len_buf[..]),
                        IoSlice::new(&rsp),
                        IoSlice::new(&data),
                    ];
                    net.send_to_vectored(from, rsp_tag, &mut iov).await.unwrap();
                });
            }
        });
    }
}
