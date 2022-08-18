//! Batteries included server and client.

pub use self::channel::{Channel, Endpoint};
pub use self::error::Error;
pub use self::server::Server;
pub use tonic::codegen::http::Uri;

pub mod channel;
mod error;
pub mod server;

/// A trait to provide a static reference to the service's
/// name. This is used for routing service's within the router.
pub trait NamedService {
    /// The `Service-Name` as described [here].
    ///
    /// [here]: https://github.com/grpc/grpc/blob/master/doc/PROTOCOL-HTTP2.md#requests
    const NAME: &'static str;
}
