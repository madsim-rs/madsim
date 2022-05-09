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
pub mod transport;

/// Codegen exports used by `madsim-tonic-build`.
pub mod codegen {
    use std::any::Any;

    pub use futures;
    pub use tonic::codegen::*;

    /// A type-erased message.
    pub type BoxMessage = Box<dyn Any + Send + Sync>;
    /// A type-erased stream of messages.
    pub type BoxMessageStream = BoxStream<BoxMessage>;
}
