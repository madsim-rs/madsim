pub mod client;
pub use client::*;
pub mod config;
pub use config::Config;
pub mod aws_endpoint;
pub mod error;
pub mod model;
pub mod no_credentials;
pub mod output;

pub use aws_smithy_http::endpoint::Endpoint;
pub use aws_types::region::Region;
pub use aws_types::Credentials;

pub mod types {
    pub use aws_smithy_http::byte_stream::AggregatedBytes;
    pub use aws_smithy_http::byte_stream::ByteStream;
    pub use aws_smithy_http::result::SdkError;
    pub use aws_smithy_types::Blob;
    pub use aws_smithy_types::DateTime;
}
