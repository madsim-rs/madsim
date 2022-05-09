pub use self::codec::Streaming;
pub use tonic::{
    async_trait, codegen, metadata, Code, IntoRequest, IntoStreamingRequest, Request, Response,
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
pub mod transport;
