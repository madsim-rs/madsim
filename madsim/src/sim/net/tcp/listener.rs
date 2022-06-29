use std::{io, net::SocketAddr};

use futures::{channel::mpsc, StreamExt};
use log::trace;
use tokio::sync::Mutex;

use super::{sim::TcpSim, to_socket_addrs, TcpStream, ToSocketAddrs};
use crate::plugin;

/// a simulated TCP socket server, listen for connections
#[cfg_attr(docsrs, doc(cfg(madsim)))]
pub struct TcpListener {
    // fixme: Maybe here no need for the mutex. but we need access 
    // the receiver without the mut and the acess will cross .await
    receiver: Mutex<mpsc::Receiver<(u64, u64)>>,
}

impl TcpListener {
    /// simulated for tokio::net::TcpListener::bind
    /// 
    /// Creates a new TcpListener, which will be bound to the specified address.
    ///
    /// The returned listener is ready for accepting connections.
    /// 
    /// The address type can be any implementor of the [`ToSocketAddrs`] trait.
    /// If `addr` yields multiple addresses, bind will be attempted with each of
    /// the addresses until one succeeds and returns the listener. If none of
    /// the addresses succeed in creating a listener, the error returned from
    /// the last attempt (the last address) is returned.
    /// 
    /// [`ToSocketAddrs`]: trait@crate::net::ToSocketAddrs
    pub async fn bind<A: ToSocketAddrs>(addr: A) -> io::Result<TcpListener> {
        let sim = plugin::simulator::<TcpSim>();
        let id = plugin::node();
        let addrs = to_socket_addrs(addr).await?;
        let mut last_err = None;

        for addr in addrs {
            sim.rand_delay().await;
            match sim.network.listen(id, addr) {
                Ok(receiver) => {
                    trace!("tcp bind to {}", addr);
                    return Ok(TcpListener {
                        receiver: Mutex::new(receiver),
                    });
                }
                Err(e) => {
                    last_err = Some(io::Error::new(io::ErrorKind::InvalidInput, e));
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

    /// simulated for tokio::net::TcpListener::accept
    /// 
    /// Accepts a new incoming connection from this listener.
    ///
    /// This function will yield once a new TCP connection is established. When
    /// established, the corresponding [`TcpStream`] and the remote peer's
    /// address will be returned.
    ///
    ///
    /// [`TcpStream`]: struct@crate::net::TcpStream
    pub async fn accept(&self) -> io::Result<(TcpStream, SocketAddr)> {
        let sim = plugin::simulator::<TcpSim>();
        sim.rand_delay().await;
        let mut receiver = self.receiver.lock().await;
        match receiver.next().await {
            Some((send_conn, recv_conn)) => {
                let tcp_stream = TcpStream {
                    send_conn,
                    recv_conn,
                };
                // fix ip selection
                trace!("tcp accepted conn({}, {})", send_conn, recv_conn);
                Ok((tcp_stream, "0.0.0.0:0".parse().unwrap()))
            }
            None => Err(io::Error::new(
                io::ErrorKind::ConnectionReset,
                "connection reset".to_string(),
            )),
        }
    }
}
