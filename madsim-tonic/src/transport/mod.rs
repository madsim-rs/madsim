//! Batteries included server and client.

pub use self::channel::{Channel, Endpoint};
pub use self::server::Server;
pub use tonic::transport::Error;

pub mod channel;
pub mod server;
