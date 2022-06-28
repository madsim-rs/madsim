use std::{io, net::SocketAddr};


use futures::{channel::mpsc, StreamExt};
use log::debug;

use crate::plugin;
use super::{TcpStream, ToSocketAddrs, to_socket_addrs, sim::TcpSim};

/// a simulated TCP socket server, listen for connections
pub struct TcpListener {
    receiver: mpsc::Receiver<(u64, u64)>
}


impl TcpListener {

    /// simulated for tokio::net::TcpListener::bind
    pub async fn bind<A: ToSocketAddrs>(addr: A) -> io::Result<TcpListener> {
        let sim = plugin::simulator::<TcpSim>();
        let id = plugin::node();
        let addrs = to_socket_addrs(addr).await?;
        let mut last_err = None;

        for addr in addrs {
            sim.rand_delay().await;
            match sim.network.listen(id, addr) {
                Ok(receiver) => {
                    debug!("tcp bind to {}", addr);
                    return Ok(TcpListener { receiver });
                }
                Err(e) => {
                    last_err = Some(io::Error::new(
                            io::ErrorKind::InvalidInput,
                            e
                    ));
                }

            }
        }
        
        Err(last_err.unwrap_or_else(|| io::Error::new(
            io::ErrorKind::AddrNotAvailable,
            "no available addr to bind".to_string()
        )))
    }

    /// simulated for tokio::net::TcpListener::accept
    pub async fn accept(&mut self) -> io::Result<(TcpStream, SocketAddr)> {
        let sim = plugin::simulator::<TcpSim>();
        sim.rand_delay().await;
        match self.receiver.next().await {
            Some((send_conn, recv_conn)) => {
                let tcp_stream = TcpStream {
                    send_conn, recv_conn
                };
                // fix ip selection
                debug!("tcp accepted conn({}, {})", send_conn, recv_conn);
                Ok((tcp_stream, "0.0.0.0:0".parse().unwrap()))
            }
            None => Err(io::Error::new(io::ErrorKind::ConnectionReset, "connection reset".to_string()))
        }
    }

}
