//! Generic client implementation.

use futures::{pin_mut, Stream, StreamExt};
use tonic::codegen::http::uri::PathAndQuery;
use tracing::instrument;

use crate::{codegen::BoxMessage, Request, Response, Status, Streaming};

#[derive(Debug, Clone)]
pub struct Grpc<T> {
    inner: T,
}

impl<T> Grpc<T> {
    /// Creates a new gRPC client with the provided `GrpcService`.
    pub fn new(inner: T) -> Self {
        Grpc { inner }
    }
}

impl Grpc<crate::transport::Channel> {
    /// Check if the inner GrpcService is able to accept a new request.
    pub async fn ready(&mut self) -> Result<(), crate::transport::Error> {
        Ok(())
    }

    /// Send a single unary gRPC request.
    #[instrument(name = "rpc", skip_all, fields(?path))]
    pub async fn unary<M1, M2, C>(
        &mut self,
        request: Request<M1>,
        path: PathAndQuery,
        _codec: C,
    ) -> Result<Response<M2>, Status>
    where
        M1: Send + Sync + 'static,
        M2: Send + Sync + 'static,
    {
        let addr = self.inner.ep.peer_addr().unwrap();
        let (tx, mut rx) = self.inner.ep.connect1(addr).await?;
        // send request
        tx.send(Box::new((path, Box::new(request) as BoxMessage)))
            .await?;
        // receive response
        let rsp = rx.recv().await?;
        let rsp = *rsp
            .downcast::<Result<BoxMessage, Status>>()
            .expect("message type mismatch");
        let rsp = *rsp?
            .downcast::<Response<M2>>()
            .expect("message type mismatch");
        Ok(rsp)
    }

    /// Send a client side streaming gRPC request.
    #[instrument(name = "rpc", skip_all, fields(?path))]
    pub async fn client_streaming<M1, M2, C>(
        &mut self,
        request: Request<impl Stream<Item = M1> + Send + 'static>,
        path: PathAndQuery,
        _codec: C,
    ) -> Result<Response<M2>, Status>
    where
        M1: Send + Sync + 'static,
        M2: Send + Sync + 'static,
    {
        let addr = self.inner.ep.peer_addr().unwrap();
        let (tx, mut rx) = self.inner.ep.connect1(addr).await?;
        // send requests
        self.send_request_stream(request, tx, path).await?;
        // receive response
        let rsp = rx.recv().await?;
        let rsp = *rsp
            .downcast::<Result<BoxMessage, Status>>()
            .expect("message type mismatch");
        let rsp = *rsp?
            .downcast::<Response<M2>>()
            .expect("message type mismatch");
        Ok(rsp)
    }

    /// Send a server side streaming gRPC request.
    #[instrument(name = "rpc", skip_all, fields(?path))]
    pub async fn server_streaming<M1, M2, C>(
        &mut self,
        request: Request<M1>,
        path: PathAndQuery,
        _codec: C,
    ) -> Result<Response<Streaming<M2>>, Status>
    where
        M1: Send + Sync + 'static,
        M2: Send + Sync + 'static,
    {
        let addr = self.inner.ep.peer_addr().unwrap();
        let (tx, rx) = self.inner.ep.connect1(addr).await?;
        // send request
        tx.send(Box::new((path, Box::new(request) as BoxMessage)))
            .await?;
        // receive responses
        Ok(Response::new(Streaming::new(rx, None)))
    }

    /// Send a bi-directional streaming gRPC request.
    #[instrument(name = "rpc", skip_all, fields(?path))]
    pub async fn streaming<M1, M2, C>(
        &mut self,
        request: Request<impl Stream<Item = M1> + Send + 'static>,
        path: PathAndQuery,
        _codec: C,
    ) -> Result<Response<Streaming<M2>>, Status>
    where
        M1: Send + Sync + 'static,
        M2: Send + Sync + 'static,
    {
        let addr = self.inner.ep.peer_addr().unwrap();
        let (tx, rx) = self.inner.ep.connect1(addr).await?;
        // send requests in a background task
        let this = self.clone();
        let task = madsim::task::spawn(async move {
            this.send_request_stream(request, tx, path).await.unwrap();
        });
        // receive responses
        Ok(Response::new(Streaming::new(rx, Some(task))))
    }

    async fn send_request_stream<M1>(
        &self,
        request: Request<impl Stream<Item = M1> + Send + 'static>,
        tx: madsim::net::Sender,
        path: PathAndQuery,
    ) -> Result<(), Status>
    where
        M1: Send + Sync + 'static,
    {
        // TODO: send `Request` metadata
        // send stream start message
        tx.send(Box::new((path, Box::new(()) as BoxMessage)))
            .await?;
        // send requests
        let stream = request.into_inner();
        pin_mut!(stream);
        while let Some(request) = stream.next().await {
            tx.send(Box::new(request) as BoxMessage).await?;
        }
        Ok(())
    }
}
