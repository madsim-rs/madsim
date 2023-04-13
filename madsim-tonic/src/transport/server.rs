//! Server implementation and builder.

use super::{Error, NamedService};
use crate::codegen::{BoxMessage, BoxMessageStream, RequestExt, ResponseExt};
use crate::sim::AppendMetadata;
use crate::tower::layer::util::{Identity, Stack};
use crate::{Request, Response, Status};
use async_stream::try_stream;
use futures_util::{future::poll_fn, select_biased, FutureExt, StreamExt};
use madsim::net::Endpoint;
use std::{
    collections::HashMap,
    future::{pending, Future},
    marker::PhantomData,
    net::SocketAddr,
    time::Duration,
};
use tonic::codegen::{http::uri::PathAndQuery, BoxFuture, Service};
#[cfg(feature = "tls")]
use tonic::transport::ServerTlsConfig;
use tracing::*;

/// A default batteries included `transport` server.
#[derive(Clone, Debug)]
pub struct Server<L = Identity> {
    _mark: PhantomData<L>,
}

#[allow(clippy::derivable_impls)]
impl Default for Server {
    fn default() -> Self {
        Self { _mark: PhantomData }
    }
}

impl Server {
    /// Create a new server builder that can configure a [`Server`].
    pub fn builder() -> Self {
        Self::default()
    }
}

impl<L> Server<L> {
    /// Create a router with the `S` typed service as the first service.
    pub fn add_service<S>(&mut self, svc: S) -> Router<L>
    where
        S: Service<
                (PathAndQuery, Request<BoxMessageStream>),
                Response = Response<BoxMessageStream>,
                Error = Status,
                Future = BoxFuture<Response<BoxMessageStream>, Status>,
            > + NamedService
            + Send
            + 'static,
        L: Clone,
    {
        let router = Router {
            server: self.clone(),
            services: Default::default(),
        };
        router.add_service(svc)
    }

    /// Set the Tower Layer all services will be wrapped in.
    pub fn layer<NewLayer>(self, _new_layer: NewLayer) -> Server<Stack<NewLayer, L>> {
        tracing::warn!("layer is unimplemented and ignored");
        Server { _mark: PhantomData }
    }

    /// Configure TLS for this server.
    #[cfg(feature = "tls")]
    #[cfg_attr(docsrs, doc(cfg(feature = "tls")))]
    pub fn tls_config(self, _tls_config: ServerTlsConfig) -> Result<Self, Error> {
        // ignore this setting
        Ok(self)
    }

    /// Set the concurrency limit applied to on requests inbound per connection.
    #[must_use]
    pub fn concurrency_limit_per_connection(self, _limit: usize) -> Self {
        // ignore this setting
        self
    }

    /// Set a timeout on for all request handlers.
    #[must_use]
    pub fn timeout(self, _timeout: Duration) -> Self {
        // ignore this setting
        self
    }

    /// Sets the `SETTINGS_INITIAL_WINDOW_SIZE` option for HTTP2 stream-level flow control.
    #[must_use]
    pub fn initial_stream_window_size(self, _sz: impl Into<Option<u32>>) -> Self {
        // ignore this setting
        self
    }

    /// Sets the max connection-level flow control for HTTP2
    #[must_use]
    pub fn initial_connection_window_size(self, _sz: impl Into<Option<u32>>) -> Self {
        // ignore this setting
        self
    }

    /// Sets the `SETTINGS_MAX_CONCURRENT_STREAMS` option for HTTP2 connections.
    #[must_use]
    pub fn max_concurrent_streams(self, _max: impl Into<Option<u32>>) -> Self {
        // ignore this setting
        self
    }

    /// Set whether HTTP2 Ping frames are enabled on accepted connections.
    #[must_use]
    pub fn http2_keepalive_interval(self, _http2_keepalive_interval: Option<Duration>) -> Self {
        // ignore this setting
        self
    }

    /// Sets a timeout for receiving an acknowledgement of the keepalive ping.
    #[must_use]
    pub fn http2_keepalive_timeout(self, _http2_keepalive_timeout: Option<Duration>) -> Self {
        // ignore this setting
        self
    }

    /// Set whether TCP keepalive messages are enabled on accepted connections.
    #[must_use]
    pub fn tcp_keepalive(self, _tcp_keepalive: Option<Duration>) -> Self {
        // ignore this setting
        self
    }

    /// Set the value of `TCP_NODELAY` option for accepted connections. Enabled by default.
    #[must_use]
    pub fn tcp_nodelay(self, _enabled: bool) -> Self {
        // ignore this setting
        self
    }

    /// Sets the maximum frame size to use for HTTP2.
    #[must_use]
    pub fn max_frame_size(self, _frame_size: impl Into<Option<u32>>) -> Self {
        // ignore this setting
        self
    }

    /// Allow this server to accept http1 requests.
    #[must_use]
    pub fn accept_http1(self, _accept_http1: bool) -> Self {
        // ignore this setting
        self
    }
}

/// A stack based `Service` router.
pub struct Router<L = Identity> {
    // TODO: support layers
    #[allow(dead_code)]
    server: Server<L>,

    #[allow(clippy::type_complexity)]
    services: HashMap<
        &'static str,
        Box<
            dyn Service<
                    (PathAndQuery, Request<BoxMessageStream>),
                    Response = Response<BoxMessageStream>,
                    Error = Status,
                    Future = BoxFuture<Response<BoxMessageStream>, Status>,
                > + Send
                + 'static,
        >,
    >,
}

impl<L> Router<L> {
    /// Add a new service to this router.
    pub fn add_service<S>(mut self, svc: S) -> Self
    where
        S: Service<
                (PathAndQuery, Request<BoxMessageStream>),
                Response = Response<BoxMessageStream>,
                Error = Status,
                Future = BoxFuture<Response<BoxMessageStream>, Status>,
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
    #[instrument(name = "server", skip(self, signal))]
    pub async fn serve_with_shutdown(
        mut self,
        addr: SocketAddr,
        signal: impl Future<Output = ()>,
    ) -> Result<(), Error> {
        let ep = Endpoint::bind(addr).await.map_err(Error::from_source)?;
        let mut signal = Box::pin(signal).fuse();
        loop {
            // receive a request
            let (tx, mut rx, addr) = select_biased! {
                ret = ep.accept1().fuse() => ret.map_err(Error::from_source)?,
                _ = &mut signal => return Ok(()),
            };
            let msg = match rx.recv().await {
                Ok(msg) => msg,
                Err(_) => continue, // maybe handshake or error
            };
            let (path, server_streaming, mut request) = *msg
                .downcast::<(PathAndQuery, bool, Request<BoxMessage>)>()
                .expect("invalid type");
            let span = debug_span!("request", ?addr, ?path);
            debug!(parent: &span, "received");

            request.set_remote_addr(addr);
            let request: Request<BoxMessageStream> = request.map(move |msg| {
                if msg.downcast_ref::<()>().is_none() {
                    // single request
                    try_stream! { yield msg; }.boxed()
                } else {
                    // request stream
                    try_stream! {
                        while let Ok(msg) = rx.recv().await {
                            yield msg;
                        }
                    }
                    .boxed()
                }
            });

            // call the service in a new spawned task
            let svc_name = path.path().split('/').nth(1).unwrap();
            let Some(svc) = &mut self.services.get_mut(svc_name) else {
                // return error Unimplemented
                madsim::task::spawn(async move {
                    let mut err = Status::unimplemented(format!("service not found: {path}"));
                    err.metadata_mut().append_metadata();
                    let msg: BoxMessage = if server_streaming {
                        Box::new(Err(err) as Result<Response<()>, Status>)
                    } else {
                        Box::new(Err(err) as Result<Response<BoxMessage>, Status>)
                    };
                    _ = tx.send(msg).await;
                });
                continue;
            };
            poll_fn(|cx| svc.poll_ready(cx)).await.unwrap();
            let rsp_future = svc.call((path, request));
            madsim::task::spawn(async move {
                let mut result: Result<Response<BoxMessageStream>, Status> =
                    rsp_future.instrument(span.clone()).await;
                result.append_metadata();
                if server_streaming {
                    let (header, stream) = match result {
                        Ok(response) => {
                            let (metadata, extensions, stream) = response.into_parts();
                            let header = Response::from_parts(metadata, extensions, ());
                            (Ok(header), Some(stream))
                        }
                        Err(e) => (Err(e), None),
                    };
                    // send the header
                    tx.send(Box::new(header)).await?;
                    // send the stream
                    let Some(mut stream) = stream else { return Ok(()) };
                    let mut count = 0;
                    loop {
                        let msg = select_biased! {
                            _ = tx.closed().fuse() => {
                                debug!(parent: &span, "client closed");
                                return Ok(());
                            }
                            msg = stream.next().fuse() => match msg {
                                Some(msg) => msg,
                                None => break,
                            }
                        };
                        // rsp: Result<BoxMessage, Status>
                        tx.send(Box::new(msg)).await?;
                        count += 1;
                    }
                    // send the trailer
                    tx.send(Box::new(())).await?;
                    debug!(parent: &span, "completed {count}");
                } else {
                    let rsp: Result<Response<BoxMessage>, Status> = match result {
                        Ok(response) => {
                            let (metadata, extensions, mut stream) = response.into_parts();
                            let inner: BoxMessage = select_biased! {
                                _ = tx.closed().fuse() => {
                                    debug!(parent: &span, "client closed");
                                    return Ok(());
                                }
                                msg = stream.next().fuse() => msg.unwrap().unwrap(),
                            };
                            Ok(Response::from_parts(metadata, extensions, inner))
                        }
                        Err(e) => Err(e),
                    };
                    // send the response
                    tx.send(Box::new(rsp)).await?;
                    debug!(parent: &span, "completed");
                }
                Ok(()) as std::io::Result<()>
            });
        }
    }
}
