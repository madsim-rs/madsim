use super::*;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{
    any::{Any, TypeId},
    fmt::Debug,
    // convert::TryInto,
    future::Future,
    mem::transmute,
};

pub trait Message: Debug + Serialize + DeserializeOwned + Any + Send + Sync {}

impl<T: Debug + Serialize + DeserializeOwned + Any + Send + Sync> Message for T {}

impl NetLocalHandle {
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

    pub async fn call<Req, Rsp>(&self, dst: SocketAddr, request: Req) -> io::Result<Rsp>
    where
        Req: Message,
        Rsp: Message,
    {
        let req_tag: u64 = unsafe { transmute(TypeId::of::<Req>()) };
        let rsp_tag = self.handle.rand.with(|rng| rng.gen::<u64>());
        // let mut data = flexbuffers::to_vec(request).unwrap();
        // data.extend_from_slice(&rsp_tag.to_ne_bytes()[..]);
        // self.send_to(dst, req_tag, &data).await?;
        self.send_to_raw(dst, req_tag, Box::new((rsp_tag, request)))
            .await?;
        let (rsp, from) = self.recv_from_raw(rsp_tag).await?;
        assert_eq!(from, dst);
        // let rsp = flexbuffers::from_slice(&rsp).unwrap();
        let rsp = *rsp.downcast::<Rsp>().expect("message type mismatch");
        Ok(rsp)
    }

    pub fn add_rpc_handler<Req, Rsp, AsyncFn, Fut>(&self, mut f: AsyncFn)
    where
        Req: Message,
        Rsp: Message,
        AsyncFn: FnMut(Req) -> Fut + Send + 'static,
        Fut: Future<Output = Rsp> + Send + 'static,
    {
        let req_tag: u64 = unsafe { transmute(TypeId::of::<Req>()) };
        let net = self.clone();
        crate::task::spawn(async move {
            loop {
                let (data, from) = net.recv_from_raw(req_tag).await.unwrap();
                let (rsp_tag, req) = *data
                    .downcast::<(u64, Req)>()
                    .expect("message type mismatch");
                // let (data, rsp_tag_bytes) = data.split_at(data.len() - 8);
                // let rsp_tag = u64::from_ne_bytes(rsp_tag_bytes.try_into().unwrap());
                // let req: Req = flexbuffers::from_slice(data).unwrap();
                let rsp_future = f(req);
                let net = net.clone();
                crate::task::spawn(async move {
                    let rsp = rsp_future.await;
                    // let data = flexbuffers::to_vec(rsp).unwrap();
                    net.send_to_raw(from, rsp_tag, Box::new(rsp)).await.unwrap();
                })
                .detach();
            }
        })
        .detach();
    }
}

#[cfg(test)]
mod tests {
    use super::NetLocalHandle;
    use crate::Runtime;

    #[test]
    fn rpc() {
        let runtime = Runtime::new();
        let addr1 = "0.0.0.1:1".parse().unwrap();
        let addr2 = "0.0.0.2:1".parse().unwrap();
        let host1 = runtime.local_handle(addr1);
        let host2 = runtime.local_handle(addr2);

        host1
            .spawn(async move {
                let net = NetLocalHandle::current();
                net.add_rpc_handler(|x: u64| async move { x + 1 });
                net.add_rpc_handler(|x: u32| async move { x + 2 });
            })
            .detach();

        let f = host2.spawn(async move {
            let net = NetLocalHandle::current();

            let rsp: u64 = net.call(addr1, 1u64).await.unwrap();
            assert_eq!(rsp, 2u64);

            let rsp: u32 = net.call(addr1, 1u32).await.unwrap();
            assert_eq!(rsp, 3u32);
        });

        runtime.block_on(f);
    }
}
