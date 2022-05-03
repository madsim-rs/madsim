//! Batteries included server and client.

pub use self::channel::{Channel, Endpoint};
pub use self::error::Error;
pub use self::server::Server;

pub mod channel;
mod error;
pub mod server;
