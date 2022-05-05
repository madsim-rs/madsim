//! Generic client implementation.

use futures::Stream;
use tonic::codegen::http::uri::PathAndQuery;

use crate::{Request, Response, Status, Streaming};

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
        todo!()
    }

    /// Send a single unary gRPC request.
    pub async fn unary<M1, M2, C>(
        &mut self,
        request: Request<M1>,
        path: PathAndQuery,
        codec: C,
    ) -> Result<Response<M2>, Status> {
        todo!()
    }

    /// Send a client side streaming gRPC request.
    pub async fn client_streaming<M1, M2, C>(
        &mut self,
        request: Request<impl Stream<Item = M1> + Send + 'static>,
        path: PathAndQuery,
        codec: C,
    ) -> Result<Response<M2>, Status> {
        todo!()
    }

    /// Send a server side streaming gRPC request.
    pub async fn server_streaming<M1, M2, C>(
        &mut self,
        request: Request<M1>,
        path: PathAndQuery,
        codec: C,
    ) -> Result<Response<Streaming<M2>>, Status> {
        todo!()
    }

    /// Send a bi-directional streaming gRPC request.
    pub async fn streaming<M1, M2, C>(
        &mut self,
        request: Request<impl Stream<Item = M1> + Send + 'static>,
        path: PathAndQuery,
        codec: C,
    ) -> Result<Response<Streaming<M2>>, Status> {
        todo!()
    }
}
