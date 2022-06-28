use super::{
    addr::{to_socket_addrs, ToSocketAddrs},
    network::ConnId,
    sim::TcpSim,
};
use crate::plugin;
use std::{
    fmt, io::{self, Write},
    pin::Pin,
    task::{Context, Poll},
};
use log::debug;
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

/// a simulated TcpStream for std::net::TcpStream
pub struct TcpStream {
    pub(crate) send_conn: ConnId,
    pub(crate) recv_conn: ConnId,
}

impl TcpStream {
    /// connect to the addr listened by another node
    pub async fn connect<A: ToSocketAddrs>(addr: A) -> io::Result<TcpStream> {
        let sim = plugin::simulator::<TcpSim>();
        let current_node = plugin::node();
        let addrs = to_socket_addrs(addr).await?;
        let mut last_err = None;

        for addr in addrs {
            // a relay before resolve an address
            sim.rand_delay().await;
            match sim.network.connect(current_node, &addr).await {
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

        Err(last_err.unwrap_or_else(|| io::Error::new(
            io::ErrorKind::AddrNotAvailable,
            "no available addr to bind".to_string()
        )))
}
}

impl AsyncRead for TcpStream {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let sim = plugin::simulator::<TcpSim>();
        match sim.network.recv(&self.recv_conn, Some(cx)) {
            Ok(Some(payload)) => {
                let data = payload.downcast::<Vec<u8>>().expect("message is not data");
                let mut b = unsafe {&mut *(buf.unfilled_mut() as *mut [std::mem::MaybeUninit<u8>] as *mut [u8])};
                match b.write(data.as_slice()) {
                    Ok(n) => {
                        unsafe {buf.assume_init(n);}
                        buf.advance(n);
                        Poll::Ready(Ok(()))
                    }
                    Err(e) => {
                        Poll::Ready(Err(io::Error::new(io::ErrorKind::ConnectionReset, e)))
                    }
                }

            }
            Ok(None) => {
                Poll::Pending
            }
            Err(e) => {
                Poll::Ready(Err(io::Error::new(io::ErrorKind::ConnectionReset, e)))
            }
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
        
        match sim.network.send(&self.send_conn, Box::new(Vec::from(buf))) {
            Ok(n) => Poll::Ready(Ok(n)),
            Err(e) => Poll::Ready(Err(io::Error::new(io::ErrorKind::ConnectionReset, e)))
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
        unimplemented!()
    }

    #[inline]
    fn poll_flush(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<io::Result<()>> {
        unimplemented!()
    }

    fn poll_shutdown(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<io::Result<()>> {
        unimplemented!();
    }
}

impl fmt::Debug for TcpStream {
    fn fmt(&self, _f: &mut fmt::Formatter<'_>) -> fmt::Result {
        unimplemented!();
    }
}
