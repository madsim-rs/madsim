//! RPC support.
//!
//! # Methods
//!
//! RPC extension adds the following methods for [`NetLocalHandle`]:
//!
//! - [`call`][NetLocalHandle::call]
//! - [`call_with_data`][NetLocalHandle::call_with_data]
//! - [`call_timeout`][NetLocalHandle::call_timeout]
//! - [`add_rpc_handler`][NetLocalHandle::add_rpc_handler]
//! - [`add_rpc_handler_with_data`][NetLocalHandle::add_rpc_handler_with_data]
//!
//! # Examples
//!
//! ```
//! # use madsim_sim as madsim;
//! use madsim::{Runtime, net::{NetLocalHandle, rpc::*}};
//!
//! let runtime = Runtime::new();
//! let host1 = runtime.create_host("0.0.0.1:1").unwrap();
//! let host2 = runtime.create_host("0.0.0.2:1").unwrap();
//! let addr1 = host1.local_addr();
//! let addr2 = host2.local_addr();
//!
//! #[derive(Serialize, Deserialize)]
//! struct Req1(u32);
//! impl Request for Req1 {
//!     type Response = u32;
//! }
//!
//! #[derive(Serialize, Deserialize)]
//! struct Req2(u32);
//! impl Request for Req2 {
//!     type Response = u32;
//! }
//!
//! host1
//!     .spawn(async move {
//!         let net = NetLocalHandle::current();
//!         net.add_rpc_handler(|x: Req1| async move { x.0 + 1 });
//!         net.add_rpc_handler_with_data(|x: Req2, data| async move {
//!             (x.0 + 2, b"hello".to_vec())
//!         });
//!     })
//!     .detach();
//!
//! let f = host2.spawn(async move {
//!     let net = NetLocalHandle::current();
//!
//!     let rsp = net.call(addr1, Req1(1)).await.unwrap();
//!     assert_eq!(rsp, 2);
//!
//!     let (rsp, data) = net.call_with_data(addr1, Req2(1), b"hi").await.unwrap();
//!     assert_eq!(rsp, 3);
//!     assert_eq!(data, &b"hello"[..]);
//! });
//!
//! runtime.block_on(f);
//! ```

use super::*;
#[doc(no_inline)]
pub use bytes::Bytes;
use futures::FutureExt;
#[doc(no_inline)]
pub use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{any::Any, future::Future};

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

impl NetLocalHandle {
    /// Call function on a remote host with timeout.
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

    /// Call function on a remote host.
    pub async fn call<R: Request>(&self, dst: SocketAddr, request: R) -> io::Result<R::Response> {
        let (rsp, _data) = self.call_with_data(dst, request, &[]).await?;
        Ok(rsp)
    }

    /// Call function on a remote host.
    pub async fn call_with_data<R: Request>(
        &self,
        dst: SocketAddr,
        request: R,
        data: &[u8],
    ) -> io::Result<(R::Response, Bytes)> {
        let req_tag = R::ID;
        let rsp_tag = self.handle.rand.with(|rng| rng.gen::<u64>());
        let data = Bytes::copy_from_slice(data);
        self.send_to_raw(dst, req_tag, Box::new((rsp_tag, request, data)))
            .await?;
        let (rsp, from) = self.recv_from_raw(rsp_tag).await?;
        assert_eq!(from, dst);
        let (rsp, data) = *rsp
            .downcast::<(R::Response, Bytes)>()
            .expect("message type mismatch");
        Ok((rsp, data))
    }

    /// Add a RPC handler.
    pub fn add_rpc_handler<R: Request, AsyncFn, Fut>(&self, mut f: AsyncFn)
    where
        AsyncFn: FnMut(R) -> Fut + Send + 'static,
        Fut: Future<Output = R::Response> + Send + 'static,
    {
        self.add_rpc_handler_with_data(move |req, _data| f(req).map(|rsp| (rsp, vec![])))
    }

    /// Add a RPC handler that send and receive data.
    pub fn add_rpc_handler_with_data<R: Request, AsyncFn, Fut>(&self, mut f: AsyncFn)
    where
        AsyncFn: FnMut(R, Bytes) -> Fut + Send + 'static,
        Fut: Future<Output = (R::Response, Vec<u8>)> + Send + 'static,
    {
        let req_tag = R::ID;
        let net = self.clone();
        crate::task::spawn(async move {
            loop {
                let (data, from) = net.recv_from_raw(req_tag).await.unwrap();
                let (rsp_tag, req, data) = *data
                    .downcast::<(u64, R, Bytes)>()
                    .expect("message type mismatch");
                let rsp_future = f(req, data);
                let net = net.clone();
                crate::task::spawn(async move {
                    let (rsp, data) = rsp_future.await;
                    net.send_to_raw(from, rsp_tag, Box::new((rsp, Bytes::from(data))))
                        .await
                        .unwrap();
                })
                .detach();
            }
        })
        .detach();
    }
}
