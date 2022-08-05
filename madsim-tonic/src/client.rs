//! Generic client implementation.

use futures::{pin_mut, Stream, StreamExt};
use madsim::rand::random;
use tonic::codegen::http::uri::PathAndQuery;

use crate::{codec::StreamEnd, codegen::BoxMessage, Request, Response, Status, Streaming};

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
        // generate a random tag for response
        let rsp_tag = random::<u64>();
        // send request
        let data = Box::new((rsp_tag, path, Box::new(request) as BoxMessage, false, false));
        self.inner.ep.send_raw(0, data).await?;
        // receive response
        let rsp = self.inner.ep.recv_raw(rsp_tag).await?;
        let rsp = *rsp
            .downcast::<Result<BoxMessage, Status>>()
            .expect("message type mismatch");
        let rsp = *rsp?
            .downcast::<Response<M2>>()
            .expect("message type mismatch");
        Ok(rsp)
    }

    /// Send a client side streaming gRPC request.
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
        // generate a random tag for request and responses
        let tag = random::<u64>();
        // send requests
        self.send_request_stream(request, tag, path, false).await?;
        // receive response
        let rsp = self.inner.ep.recv_raw(tag).await?;
        let rsp = *rsp
            .downcast::<Result<BoxMessage, Status>>()
            .expect("message type mismatch");
        let rsp = *rsp?
            .downcast::<Response<M2>>()
            .expect("message type mismatch");
        Ok(rsp)
    }

    /// Send a server side streaming gRPC request.
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
        // generate a random tag for responses
        let rsp_tag = random::<u64>();
        // send request
        let data = Box::new((rsp_tag, path, Box::new(request) as BoxMessage, false, true));
        self.inner.ep.send_raw(0, data).await?;
        // receive responses
        Ok(Response::new(Streaming::new(
            self.inner.ep.clone(),
            rsp_tag,
            None,
        )))
    }

    /// Send a bi-directional streaming gRPC request.
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
        // generate a random tag for requests and responses
        let tag = random::<u64>();
        // send requests in a background task
        let this = self.clone();
        let task = madsim::task::spawn(async move {
            this.send_request_stream(request, tag, path, true)
                .await
                .unwrap();
        });
        // receive responses
        Ok(Response::new(Streaming::new(
            self.inner.ep.clone(),
            tag,
            Some(task),
        )))
    }

    async fn send_request_stream<M1>(
        &self,
        request: Request<impl Stream<Item = M1> + Send + 'static>,
        mut tag: u64,
        path: PathAndQuery,
        server_stream: bool,
    ) -> Result<(), Status>
    where
        M1: Send + Sync + 'static,
    {
        // TODO: send `Request` metadata
        // send stream start message
        let data = Box::new((tag, path, Box::new(()) as BoxMessage, true, server_stream));
        self.inner.ep.send_raw(0, data).await?;
        // send requests with increasing tag
        let stream = request.into_inner();
        pin_mut!(stream);
        while let Some(request) = stream.next().await {
            let data = Box::new(request);
            self.inner.ep.send_raw(tag, data).await?;
            tag += 1;
        }
        // send stream end message
        let data = Box::new(StreamEnd);
        self.inner.ep.send_raw(tag, data).await?;
        Ok(())
    }
}
