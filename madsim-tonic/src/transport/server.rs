//! Server implementation and builder.

use super::Error;
use futures::{future::poll_fn, select_biased, FutureExt};
use madsim::net::Endpoint;
use std::{
    any::Any,
    convert::Infallible,
    future::{pending, Future},
    net::SocketAddr,
    sync::Arc,
};
use tonic::codegen::{http::uri::PathAndQuery, BoxFuture, Service};

type BoxMessage = Box<dyn Any + Send + Sync>;

/// A default batteries included `transport` server.
#[derive(Default)]
pub struct Server {}

impl Server {
    /// Create a new server builder that can configure a [`Server`].
    pub fn builder() -> Self {
        Self::default()
    }

    /// Create a router with the `S` typed service as the first service.
    pub fn add_service<S>(&mut self, svc: S) -> Router
    where
        S: Service<
                (PathAndQuery, BoxMessage),
                Response = BoxMessage,
                Error = Infallible,
                Future = BoxFuture<BoxMessage, Infallible>,
            > + Send
            + 'static,
    {
        Router {
            services: vec![Box::new(svc)],
        }
    }
}

/// A stack based `Service` router.
pub struct Router {
    services: Vec<
        Box<
            dyn Service<
                    (PathAndQuery, BoxMessage),
                    Response = BoxMessage,
                    Error = Infallible,
                    Future = BoxFuture<BoxMessage, Infallible>,
                > + Send
                + 'static,
        >,
    >,
}

impl Router {
    /// Add a new service to this router.
    pub fn add_service<S>(mut self, svc: S) -> Self
    where
        S: Service<
                (PathAndQuery, BoxMessage),
                Response = BoxMessage,
                Error = Infallible,
                Future = BoxFuture<BoxMessage, Infallible>,
            > + Send
            + 'static,
    {
        self.services.push(Box::new(svc));
        self
    }

    /// Consume this [`Server`] creating a future that will execute the server
    /// on default executor.
    pub async fn serve(self, addr: SocketAddr) -> Result<(), Error> {
        self.serve_with_shutdown(addr, pending::<()>()).await
    }

    /// Consume this [`Server`] creating a future that will execute the server
    /// on default executor. And shutdown when the provided signal is received.
    pub async fn serve_with_shutdown(
        mut self,
        addr: SocketAddr,
        signal: impl Future<Output = ()>,
    ) -> Result<(), Error> {
        let ep = Arc::new(Endpoint::bind(addr).await.map_err(Error::from_source)?);
        let mut signal = Box::pin(signal).fuse();
        loop {
            let (msg, from) = select_biased! {
                ret = ep.recv_from_raw(0).fuse() => ret.map_err(Error::from_source)?,
                _ = &mut signal => return Ok(()),
            };
            let (rsp_tag, path, msg) = *msg
                .downcast::<(u64, PathAndQuery, BoxMessage)>()
                .expect("invalid type");
            let svc = &mut self.services[0];
            poll_fn(|cx| svc.poll_ready(cx))
                .await
                .map_err(Error::from_source)?;
            let rsp_future = svc.call((path, msg));
            let ep = ep.clone();
            madsim::task::spawn(async move {
                let rsp = rsp_future.await.unwrap();
                ep.send_to_raw(from, rsp_tag, rsp)
                    .await
                    .expect("failed to send response");
            })
            .detach();
        }
    }
}
