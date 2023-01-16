pub mod client;
pub use client::*;
pub mod server;

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