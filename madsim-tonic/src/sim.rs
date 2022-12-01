pub use self::codec::Streaming;
pub use tonic::{
    async_trait, metadata, service, Code, IntoRequest, IntoStreamingRequest, Request, Response,
    Status,
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

/// Codegen exports used by `madsim-tonic-build`.
pub mod codegen {
    use std::any::Any;
    pub use std::net::SocketAddr;
    use tonic::Status;

    pub use futures_util as futures;
    pub use tonic::codegen::*;

    /// A type-erased message.
    pub type BoxMessage = Box<dyn Any + Send + Sync>;
    /// A type-erased stream of messages.
    pub type BoxMessageStream = BoxStream<BoxMessage>;
    /// An identity interceptor.
    pub type IdentityInterceptor = fn(tonic::Request<()>) -> Result<tonic::Request<()>, Status>;

    pub trait RequestExt: Sized {
        fn set_remote_addr(&mut self, addr: SocketAddr);
        fn intercept<F: tonic::service::Interceptor>(
            self,
            interceptor: &mut F,
        ) -> Result<Self, Status>;
    }

    impl<T> RequestExt for tonic::Request<T> {
        /// Set the remote address of Request.
        fn set_remote_addr(&mut self, addr: SocketAddr) {
            let tcp_info: tonic::transport::server::TcpConnectInfo =
                unsafe { std::mem::transmute(Some(addr)) };
            self.extensions_mut().insert(tcp_info);
        }

        /// Intercept the request.
        fn intercept<F: tonic::service::Interceptor>(
            self,
            interceptor: &mut F,
        ) -> Result<Self, Status> {
            let (metadata, extensions, inner) = self.into_parts();
            let request = tonic::Request::from_parts(metadata, extensions, ());
            let request = interceptor.call(request)?;
            let (metadata, extensions, _) = request.into_parts();
            Ok(Self::from_parts(metadata, extensions, inner))
        }
    }
}
