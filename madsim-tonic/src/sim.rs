pub use self::codec::Streaming;
pub use tonic::{async_trait, include_proto, metadata, Code, Status};

pub mod client;
pub mod codec;
pub mod codegen;
pub mod transport;

#[derive(Debug)]
pub struct Request<T> {
    message: T,
}

impl<T> Request<T> {
    /// Create a new gRPC request.
    pub fn new(message: T) -> Self {
        Request { message }
    }
}

/// A gRPC response and metadata from an RPC call.
#[derive(Debug)]
pub struct Response<T> {
    message: T,
}

impl<T> Response<T> {
    /// Create a new gRPC response.
    pub fn new(message: T) -> Self {
        Response { message }
    }
}
