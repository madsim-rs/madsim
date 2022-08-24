use std::{io::Result, path::Path};

/// An I/O object representing a Unix datagram socket.
#[doc(hidden)]
#[derive(Debug)]
pub struct UnixDatagram {}

impl UnixDatagram {
    /// Creates a new [`UnixDatagram`] bound to the specified path.
    pub fn bind<P>(path: P) -> Result<UnixDatagram>
    where
        P: AsRef<Path>,
    {
        todo!()
    }

    /// Connects the socket to the specified address.
    pub fn connect<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        todo!()
    }
}
