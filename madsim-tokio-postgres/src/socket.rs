use std::io;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio::net::TcpStream;
#[cfg(all(unix, not(madsim)))]
use tokio::net::UnixStream;

#[derive(Debug)]
enum Inner {
    Tcp(TcpStream),
    #[cfg(all(unix, not(madsim)))]
    Unix(UnixStream),
}

/// The standard stream type used by the crate.
///
/// Requires the `runtime` Cargo feature (enabled by default).
#[derive(Debug)]
pub struct Socket(Inner);

impl Socket {
    pub(crate) fn new_tcp(stream: TcpStream) -> Socket {
        Socket(Inner::Tcp(stream))
    }

    #[cfg(all(unix, not(madsim)))]
    pub(crate) fn new_unix(stream: UnixStream) -> Socket {
        Socket(Inner::Unix(stream))
    }
}

impl AsyncRead for Socket {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        match &mut self.0 {
            Inner::Tcp(s) => Pin::new(s).poll_read(cx, buf),
            #[cfg(all(unix, not(madsim)))]
            Inner::Unix(s) => Pin::new(s).poll_read(cx, buf),
        }
    }
}

impl AsyncWrite for Socket {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        match &mut self.0 {
            Inner::Tcp(s) => Pin::new(s).poll_write(cx, buf),
            #[cfg(all(unix, not(madsim)))]
            Inner::Unix(s) => Pin::new(s).poll_write(cx, buf),
        }
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        match &mut self.0 {
            Inner::Tcp(s) => Pin::new(s).poll_flush(cx),
            #[cfg(all(unix, not(madsim)))]
            Inner::Unix(s) => Pin::new(s).poll_flush(cx),
        }
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        match &mut self.0 {
            Inner::Tcp(s) => Pin::new(s).poll_shutdown(cx),
            #[cfg(all(unix, not(madsim)))]
            Inner::Unix(s) => Pin::new(s).poll_shutdown(cx),
        }
    }
}
