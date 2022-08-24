//! Unix domain socket utility types.

#![cfg(unix)]
#![allow(unused)]
#![doc(hidden)]

mod datagram;
mod stream;

pub use self::datagram::UnixDatagram;
pub use self::stream::{UnixListener, UnixStream};
