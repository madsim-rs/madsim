//! Generic client implementation.

use std::future::Future;
use std::time::Duration;

use futures_util::{pin_mut, Stream, StreamExt};
use tonic::codegen::http::uri::PathAndQuery;
use tracing::instrument;

use crate::{
    codegen::{BoxMessage, IdentityInterceptor, RequestExt},
    service::Interceptor,
    sim::AppendMetadata,
    Request, Response, Status, Streaming,
};

#[derive(Debug, Clone)]
pub struct Grpc<T, F> {
    inner: T,
    interceptor: F,
}

impl<T> Grpc<T, IdentityInterceptor> {
    /// Creates a new gRPC client with the provided `GrpcService`.
    pub fn new(inner: T) -> Self {
        Grpc {
            inner,
            interceptor: Ok,
        }
    }
}

// Message type matrix:
// |          |                single                  |                   stream                     |
// |----------|----------------------------------------|----------------------------------------------|
// | request  | (PathAndQuery, bool, Request<Box<M1>>) | (PathAndQuery, bool, Request<Box<()>>), M1.. |
// | response | Result<Response<Box<M2>>>              | Result<Response<()>>, Result<Box<M2>>.., ()  |
//
impl<F: Interceptor> Grpc<crate::transport::Channel, F> {
    /// Creates a new gRPC client with the provided `GrpcService` and interceptor.
    pub fn with_interceptor(inner: crate::transport::Channel, interceptor: F) -> Self {
        Grpc { inner, interceptor }
    }

    /// Check if the inner GrpcService is able to accept a new request.
    pub async fn ready(&mut self) -> Result<(), crate::transport::Error> {
        Ok(())
    }

    /// Send a single unary gRPC request.
    #[instrument(name = "rpc", skip_all, fields(?path))]
    pub async fn unary<M1, M2, C>(
        &mut self,
        mut request: Request<M1>,
        path: PathAndQuery,
        _codec: C,
    ) -> Result<Response<M2>, Status>
    where
        M1: Send + Sync + 'static,
        M2: Send + Sync + 'static,
    {
        let timeout = self.inner.timeout;
        let future = async move {
            request.append_metadata();
            let request = request.intercept(&mut self.interceptor)?.boxed();
            let addr = self.inner.ep.peer_addr().unwrap();
            let (tx, mut rx) = self.inner.ep.connect1(addr).await?;
            // send request
            tx.send(Box::new((path, false, request))).await?;
            // receive response
            let rsp = rx.recv().await?;
            let rsp = *rsp
                .downcast::<Result<Response<BoxMessage>, Status>>()
                .expect("message type mismatch");
            let rsp = rsp?.map(|msg| *msg.downcast().expect("message type mismatch"));
            Ok(rsp)
        };
        with_timeout(timeout, future).await
    }

    /// Send a client side streaming gRPC request.
    #[instrument(name = "rpc", skip_all, fields(?path))]
    pub async fn client_streaming<M1, M2, C>(
        &mut self,
        mut request: Request<impl Stream<Item = M1> + Send + 'static>,
        path: PathAndQuery,
        _codec: C,
    ) -> Result<Response<M2>, Status>
    where
        M1: Send + Sync + 'static,
        M2: Send + Sync + 'static,
    {
        let timeout = self.inner.timeout;
        let future = async move {
            request.append_metadata();
            let request = request.intercept(&mut self.interceptor)?;
            let addr = self.inner.ep.peer_addr().unwrap();
            let (tx, mut rx) = self.inner.ep.connect1(addr).await?;
            // send requests
            Self::send_request_stream(request, tx, path, false).await?;
            // receive response
            let rsp = rx.recv().await?;
            let rsp = *rsp
                .downcast::<Result<Response<BoxMessage>, Status>>()
                .expect("message type mismatch");
            let rsp = rsp?.map(|msg| *msg.downcast().expect("message type mismatch"));
            Ok(rsp)
        };
        with_timeout(timeout, future).await
    }

    /// Send a server side streaming gRPC request.
    #[instrument(name = "rpc", skip_all, fields(?path))]
    pub async fn server_streaming<M1, M2, C>(
        &mut self,
        mut request: Request<M1>,
        path: PathAndQuery,
        _codec: C,
    ) -> Result<Response<Streaming<M2>>, Status>
    where
        M1: Send + Sync + 'static,
        M2: Send + Sync + 'static,
    {
        let timeout = self.inner.timeout;
        let future = async move {
            request.append_metadata();
            let request = request.intercept(&mut self.interceptor)?.boxed();
            let addr = self.inner.ep.peer_addr().unwrap();
            let (tx, mut rx) = self.inner.ep.connect1(addr).await?;
            // send request
            tx.send(Box::new((path, true, request))).await?;
            // receive responses
            let res = *(rx.recv().await?)
                .downcast::<Result<Response<()>, Status>>()
                .unwrap();
            let response = res?.map(move |_| Streaming::new(rx, None));
            Ok(response)
        };
        with_timeout(timeout, future).await
    }

    /// Send a bi-directional streaming gRPC request.
    #[instrument(name = "rpc", skip_all, fields(?path))]
    pub async fn streaming<M1, M2, C>(
        &mut self,
        mut request: Request<impl Stream<Item = M1> + Send + 'static>,
        path: PathAndQuery,
        _codec: C,
    ) -> Result<Response<Streaming<M2>>, Status>
    where
        M1: Send + Sync + 'static,
        M2: Send + Sync + 'static,
    {
        let timeout = self.inner.timeout;
        let future = async move {
            request.append_metadata();
            let request = request.intercept(&mut self.interceptor)?;
            let addr = self.inner.ep.peer_addr().unwrap();
            let (tx, mut rx) = self.inner.ep.connect1(addr).await?;
            // send requests in a background task
            let task = madsim::task::spawn(async move {
                _ = Self::send_request_stream(request, tx, path, true).await;
            });
            // receive responses
            let res = *(rx.recv().await?)
                .downcast::<Result<Response<()>, Status>>()
                .unwrap();
            let response = res?.map(move |_| Streaming::new(rx, Some(task)));
            Ok(response)
        };
        with_timeout(timeout, future).await
    }

    async fn send_request_stream<M1>(
        request: Request<impl Stream<Item = M1> + Send + 'static>,
        tx: madsim::net::Sender,
        path: PathAndQuery,
        server_streaming: bool,
    ) -> Result<(), Status>
    where
        M1: Send + Sync + 'static,
    {
        let (metadata, extensions, stream) = request.into_parts();
        let header = Request::from_parts(metadata, extensions, Box::new(()) as BoxMessage);
        // send stream start message
        tx.send(Box::new((path, server_streaming, header))).await?;
        // send requests
        pin_mut!(stream);
        while let Some(item) = stream.next().await {
            tx.send(Box::new(item)).await?;
        }
        Ok(())
    }
}

async fn with_timeout<T>(
    timeout: Option<Duration>,
    future: impl Future<Output = Result<T, Status>>,
) -> Result<T, Status> {
    if let Some(timeout) = timeout {
        madsim::time::timeout(timeout, future)
            .await
            .map_err(|_| Status::deadline_exceeded(format!("request timeout: {timeout:?}")))?
    } else {
        future.await
    }
}
