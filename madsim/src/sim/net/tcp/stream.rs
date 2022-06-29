use super::{
    addr::{to_socket_addrs, ToSocketAddrs},
    network::ConnId,
    sim::TcpSim,
};
use crate::plugin;
use std::{
    io::{self, Write},
    pin::Pin,
    task::{Context, Poll},
};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

/// a simulated TcpStream for std::net::TcpStream
#[derive(Debug)]
pub struct TcpStream {
    send_conn: ConnId,
    recv_conn: ConnId,
}

impl TcpStream {
    /// Opens a simulated TCP connection to a remote host.
    ///
    /// `addr` is an address of the remote host. Anything which implements the
    /// [`ToSocketAddrs`] trait can be supplied as the address.  If `addr`
    /// yields multiple addresses, connect will be attempted with each of the
    /// addresses until a connection is successful. If none of the addresses
    /// result in a successful connection, the error returned from the last
    /// connection attempt (the last address) is returned.
    ///
    /// [`ToSocketAddrs`]: trait@crate::net::ToSocketAddrs
    pub async fn connect<A: ToSocketAddrs>(addr: A) -> io::Result<TcpStream> {
        let sim = plugin::simulator::<TcpSim>();
        let current_node = plugin::node();
        let addrs = to_socket_addrs(addr).await?;
        let mut last_err = None;

        for addr in addrs {
            // a relay before resolve an address
            sim.rand_delay().await;
            match sim.network().connect(current_node, &addr).await {
                Ok((send_conn, recv_conn)) => {
                    return Ok(TcpStream {
                        send_conn,
                        recv_conn,
                    })
                }
                Err(e) => {
                    last_err = Some(io::Error::new(
                        io::ErrorKind::AddrNotAvailable,
                        format!("there is no remote listened for {}", e),
                    ));
                }
            }
        }

        Err(last_err.unwrap_or_else(|| {
            io::Error::new(
                io::ErrorKind::AddrNotAvailable,
                "no available addr to bind".to_string(),
            )
        }))
    }

    pub(crate) fn new(send_conn: ConnId, recv_conn: ConnId) -> Self {
        Self {
            send_conn,
            recv_conn,
        }
    }
}

impl AsyncRead for TcpStream {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let sim = plugin::simulator::<TcpSim>();
        match sim.network().recv(&self.recv_conn, Some(cx)) {
            Ok(Some(payload)) => {
                let data = payload.downcast::<Vec<u8>>().expect("message is not data");
                let mut b = unsafe {
                    &mut *(buf.unfilled_mut() as *mut [std::mem::MaybeUninit<u8>] as *mut [u8])
                };
                match b.write(data.as_slice()) {
                    Ok(n) => {
                        unsafe {
                            buf.assume_init(n);
                        }
                        buf.advance(n);
                        Poll::Ready(Ok(()))
                    }
                    Err(e) => Poll::Ready(Err(io::Error::new(io::ErrorKind::ConnectionReset, e))),
                }
            }
            Ok(None) => Poll::Pending,
            Err(e) => Poll::Ready(Err(io::Error::new(io::ErrorKind::ConnectionReset, e))),
        }
    }
}

impl AsyncWrite for TcpStream {
    fn poll_write(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        // self.poll_write_priv(cx, buf)
        let sim = plugin::simulator::<TcpSim>();

        match sim
            .network()
            .send(&self.send_conn, Box::new(Vec::from(buf)))
        {
            Ok(n) => Poll::Ready(Ok(n)),
            Err(e) => Poll::Ready(Err(io::Error::new(io::ErrorKind::ConnectionReset, e))),
        }
    }

    fn poll_write_vectored(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        _bufs: &[io::IoSlice<'_>],
    ) -> Poll<io::Result<usize>> {
        unimplemented!()
    }

    fn is_write_vectored(&self) -> bool {
        false
    }

    #[inline]
    fn poll_flush(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<io::Result<()>> {
        let sim = plugin::simulator::<TcpSim>();
        Poll::Ready(sim.network().drop_connection(self.send_conn))
    }
}
