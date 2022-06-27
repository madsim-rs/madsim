use std::{io, net::SocketAddr};


use futures::{channel::mpsc, StreamExt};
use log::debug;

use crate::{task::NodeId, plugin};
use super::{TcpStream, ToSocketAddrs, to_socket_addrs, sim::TcpSim};

/// a simulated TCP socket server, listen for connections
pub struct TcpListener {
    id: NodeId,
    receiver: mpsc::Receiver<(u64, u64)>
}


impl TcpListener {

    /// simulated for tokio::net::TcpListener::bind
    pub async fn bind<A: ToSocketAddrs>(addr: A) -> io::Result<TcpListener> {
        let sim = plugin::simulator::<TcpSim>();
        let id = plugin::node();
        let addr = to_socket_addrs(addr).await?.next().unwrap();
        sim.rand_delay().await;
        let receiver = sim
            .network
            .listen(id, addr)
            .map_err(|e| 
                io::Error::new(
                    io::ErrorKind::InvalidInput,
                    e
                )
            )?;
        debug!("tcp bind to {}", addr);
        Ok(TcpListener { id, receiver })
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
                Ok((tcp_stream, "10.0.0.1:1".parse().unwrap()))
            }
            None => Err(io::Error::new(io::ErrorKind::ConnectionReset, "connection reset".to_string()))
        }
    }

}
