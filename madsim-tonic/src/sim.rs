pub use self::codec::Streaming;
pub use tonic::{
    async_trait, metadata, service, Code, Extensions, IntoRequest, IntoStreamingRequest, Request,
    Response, Status,
};

#[macro_export]
macro_rules! include_proto {
    ($package: tt) => {
        include!(concat!(env!("OUT_DIR"), concat!("/sim/", $package, ".rs")));
    };
}

pub mod client;
pub mod codec;
pub(crate) mod tower;
pub mod transport;

/// Append header to metadata.
trait AppendMetadata {
    fn append_metadata(&mut self);
}
impl AppendMetadata for metadata::MetadataMap {
    fn append_metadata(&mut self) {
        self.append("content-type", "application/grpc".parse().unwrap());
        self.append("date", chrono::Utc::now().to_rfc2822().parse().unwrap());
    }
}
impl<T> AppendMetadata for Request<T> {
    fn append_metadata(&mut self) {
        let metadata = self.metadata_mut();
        metadata.append("content-type", "application/grpc".parse().unwrap());
    }
}
impl<T> AppendMetadata for Result<Response<T>, Status> {
    fn append_metadata(&mut self) {
        match self {
            Ok(r) => r.metadata_mut().append_metadata(),
            Err(e) => e.metadata_mut().append_metadata(),
        }
    }
}

/// Codegen exports used by `madsim-tonic-build`.
pub mod codegen {
    use std::any::Any;
    pub use std::net::SocketAddr;
    use tonic::{service::Interceptor, Request, Status};

    pub use futures_util as futures;
    pub use tonic::codegen::*;

    /// A type-erased message.
    pub type BoxMessage = Box<dyn Any + Send + Sync>;
    /// A type-erased stream of messages.
    pub type BoxMessageStream = BoxStream<BoxMessage>;
    /// An identity interceptor.
    pub type IdentityInterceptor = fn(Request<()>) -> Result<Request<()>, Status>;

    pub trait RequestExt<T>: Sized {
        fn set_tcp_connect_info(&mut self, local_addr: SocketAddr, remote_addr: SocketAddr);
        fn intercept<F: Interceptor>(self, interceptor: &mut F) -> Result<Self, Status>;
        fn boxed(self) -> Request<BoxMessage>
        where
            T: Send + Sync + 'static;
    }

    impl<T> RequestExt<T> for Request<T> {
        /// Set the remote address of Request.
        fn set_tcp_connect_info(&mut self, local_addr: SocketAddr, remote_addr: SocketAddr) {
            let tcp_info: tonic::transport::server::TcpConnectInfo =
                unsafe { std::mem::transmute((Some(local_addr), Some(remote_addr))) };
            self.extensions_mut().insert(tcp_info);
        }

        /// Intercept the request.
        fn intercept<F: Interceptor>(self, interceptor: &mut F) -> Result<Self, Status> {
            let (metadata, extensions, inner) = self.into_parts();
            let request = Request::from_parts(metadata, extensions, ());
            let request = interceptor.call(request)?;
            let (metadata, extensions, _) = request.into_parts();
            Ok(Self::from_parts(metadata, extensions, inner))
        }

        fn boxed(self) -> Request<BoxMessage>
        where
            T: Send + Sync + 'static,
        {
            self.map(|inner| Box::new(inner) as BoxMessage)
        }
    }
}
