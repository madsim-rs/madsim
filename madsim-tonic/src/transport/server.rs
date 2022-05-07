//! Server implementation and builder.

use super::Error;
use futures::{future::poll_fn, select_biased, FutureExt};
use madsim::net::Endpoint;
use std::{
    any::Any,
    collections::HashMap,
    convert::Infallible,
    future::{pending, Future},
    net::SocketAddr,
    sync::Arc,
};
use tonic::{
    codegen::{http::uri::PathAndQuery, BoxFuture, Service},
    transport::NamedService,
};

/// A type-erased message.
pub(crate) type BoxMessage = Box<dyn Any + Send + Sync>;

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
            > + NamedService
            + Send
            + 'static,
    {
        let router = Router {
            services: Default::default(),
        };
        router.add_service(svc)
    }
}

/// A stack based `Service` router.
pub struct Router {
    services: HashMap<
        &'static str,
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
            > + NamedService
            + Send
            + 'static,
    {
        self.services.insert(S::NAME, Box::new(svc));
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
            // receive a request
            let (msg, from) = select_biased! {
                ret = ep.recv_from_raw(0).fuse() => ret.map_err(Error::from_source)?,
                _ = &mut signal => return Ok(()),
            };
            let (rsp_tag, path, msg) = *msg
                .downcast::<(u64, PathAndQuery, BoxMessage)>()
                .expect("invalid type");
            log::trace!("request: {path} <- {from}");

            // call the service in a new spawned task
            // TODO: handle error
            let svc_name = path.path().split('/').nth(1).unwrap();
            let svc = &mut self.services.get_mut(svc_name).unwrap();
            poll_fn(|cx| svc.poll_ready(cx)).await.unwrap();
            let rsp_future = svc.call((path, msg));
            let ep = ep.clone();
            madsim::task::spawn(async move {
                let rsp = rsp_future.await.unwrap();
                // send the response
                ep.send_to_raw(from, rsp_tag, rsp)
                    .await
                    .expect("failed to send response");
            })
            .detach();
        }
    }
}
