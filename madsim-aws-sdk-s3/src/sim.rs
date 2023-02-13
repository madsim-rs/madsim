pub mod client;
pub use client::*;
pub mod server;

pub use aws_sdk_s3::{error, types};

pub use aws_smithy_http::endpoint::Endpoint;
pub use aws_types::region::Region;
pub use aws_types::Credentials;
