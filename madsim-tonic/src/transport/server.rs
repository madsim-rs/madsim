//! Server implementation and builder.

use super::Error;
use crate::{
    codec::StreamEnd,
    codegen::{BoxMessage, BoxMessageStream},
};
use async_stream::try_stream;
use futures::{future::poll_fn, select_biased, FutureExt, StreamExt};
use madsim::net::Endpoint;
use std::{
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
                (PathAndQuery, BoxMessageStream),
                Response = BoxMessageStream,
                Error = Infallible,
                Future = BoxFuture<BoxMessageStream, Infallible>,
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
                    (PathAndQuery, BoxMessageStream),
                    Response = BoxMessageStream,
                    Error = Infallible,
                    Future = BoxFuture<BoxMessageStream, Infallible>,
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
                (PathAndQuery, BoxMessageStream),
                Response = BoxMessageStream,
                Error = Infallible,
                Future = BoxFuture<BoxMessageStream, Infallible>,
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
            let (mut tag, path, msg, client_stream, server_stream) = *msg
                .downcast::<(u64, PathAndQuery, BoxMessage, bool, bool)>()
                .expect("invalid type");
            log::trace!("request: {path} <- {from}");

            let requests: BoxMessageStream = if !client_stream {
                // single request
                futures::stream::once(async move { Ok(msg) }).boxed()
            } else {
                // request stream
                let ep = ep.clone();
                try_stream! {
                    for tag in tag.. {
                        let (msg, _) = ep.recv_from_raw(tag).await?;
                        if msg.downcast_ref::<StreamEnd>().is_some() {
                            return;
                        }
                        yield msg;
                    }
                }
                .boxed()
            };

            // call the service in a new spawned task
            // TODO: handle error
            let svc_name = path.path().split('/').nth(1).unwrap();
            let svc = &mut self.services.get_mut(svc_name).unwrap();
            poll_fn(|cx| svc.poll_ready(cx)).await.unwrap();
            let rsp_future = svc.call((path, requests));
            let ep = ep.clone();
            madsim::task::spawn(async move {
                let mut stream = rsp_future.await.unwrap();
                // send the response
                while let Some(rsp) = stream.next().await {
                    ep.send_to_raw(from, tag, rsp.unwrap())
                        .await
                        .expect("failed to send response");
                    tag += 1;
                }
                if server_stream {
                    ep.send_to_raw(from, tag, Box::new(StreamEnd))
                        .await
                        .expect("failed to send response");
                }
            })
            .detach();
        }
    }
}
