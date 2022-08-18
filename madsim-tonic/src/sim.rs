pub use self::codec::Streaming;
pub use tonic::{
    async_trait, metadata, Code, IntoRequest, IntoStreamingRequest, Request, Response, Status,
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

    pub use futures_util as futures;
    pub use tonic::codegen::*;

    /// A type-erased message.
    pub type BoxMessage = Box<dyn Any + Send + Sync>;
    /// A type-erased stream of messages.
    pub type BoxMessageStream = BoxStream<BoxMessage>;

    pub trait RequestExt {
        fn set_remote_addr(&mut self, addr: SocketAddr);
    }

    impl<T> RequestExt for tonic::Request<T> {
        /// Set the remote address of Request.
        fn set_remote_addr(&mut self, addr: SocketAddr) {
            let tcp_info: tonic::transport::server::TcpConnectInfo =
                unsafe { std::mem::transmute(Some(addr)) };
            self.extensions_mut().insert(tcp_info);
        }
    }
}
