//! a tokio Tcp Simulatior
//!
//! # Examples
//!
//! ```
//!
//! use madsim::{net::{NetSim, TcpStream, TcpListener}, plugin, runtime::Runtime, time::timeout};
//! use std::{net::SocketAddr, sync::Arc, time::Duration};
//! use tokio::{
//!     io::{AsyncReadExt, AsyncWriteExt},
//!     sync::Barrier,
//! };
//!
//!
//! let runtime = Runtime::new();
//! let addr1 = "10.0.0.1:1".parse::<SocketAddr>().unwrap();
//! let addr2 = "10.0.0.2:1".parse::<SocketAddr>().unwrap();
//! let node1 = runtime.create_node().ip(addr1.ip()).build();
//! let node2 = runtime.create_node().ip(addr2.ip()).build();
//! let barrier = Arc::new(Barrier::new(2));
//!
//! let barrier_ = barrier.clone();
//! let f1 = node1.spawn(async move {
//!     let listener = TcpListener::bind(addr1).await.unwrap();
//!     barrier_.wait().await;
//!     let (mut stream, _) = listener.accept().await.unwrap();
//!     stream.write(b"hello world").await.unwrap();
//!     stream.flush().await.unwrap();
//!     stream
//! });
//!
//! let f2 = node2.spawn(async move {
//!     barrier.wait().await;
//!     let mut stream = TcpStream::connect(addr1).await.unwrap();
//!     let mut buf = [0; 20];
//!     let len = stream.read(&mut buf).await.unwrap();
//!     assert_eq!(&buf[0..len], b"hello world");
//! });
//!
//! runtime.block_on(f1).unwrap();
//! runtime.block_on(f2).unwrap();
//! ```

use std::any::Any;

/// tcp packet payload
pub type Payload = Box<dyn Any + Send + Sync>;

mod config;
mod listener;
mod stream;

pub use self::config::*;
pub use self::listener::*;
pub use self::stream::*;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        net::{ipvs::*, NetSim},
        plugin,
        runtime::Runtime,
        time::timeout,
    };
    use std::{io::ErrorKind, net::SocketAddr, sync::Arc, time::Duration};
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        sync::Barrier,
    };

    #[test]
    fn send_recv() {
        let runtime = Runtime::new();
        let addr1 = "10.0.0.1:1".parse::<SocketAddr>().unwrap();
        let addr2 = "10.0.0.2:1".parse::<SocketAddr>().unwrap();
        let node1 = runtime.create_node().ip(addr1.ip()).build();
        let node2 = runtime.create_node().ip(addr2.ip()).build();
        let barrier = Arc::new(Barrier::new(2));
        let barrier_ = barrier.clone();

        let f1 = node1.spawn(async move {
            let listener = TcpListener::bind(addr1).await.unwrap();
            barrier_.wait().await;
            let (mut stream, _) = listener.accept().await.unwrap();
            stream.write_all(b"hello world").await.unwrap();
            stream.flush().await.unwrap();
            stream
        });

        let f2 = node2.spawn(async move {
            barrier.wait().await;
            let mut stream = TcpStream::connect(addr1).await.unwrap();
            let mut buf = [0; 20];
            let len = stream.read(&mut buf).await.unwrap();
            assert_eq!(&buf[0..len], b"hello world");
        });

        runtime.block_on(f1).unwrap();
        runtime.block_on(f2).unwrap();
    }

    #[test]
    fn disconnect_and_recovery() {
        let runtime = Runtime::new();
        let addr1 = "10.0.0.1:1".parse::<SocketAddr>().unwrap();
        let addr2 = "10.0.0.2:1".parse::<SocketAddr>().unwrap();
        let node1 = runtime.create_node().ip(addr1.ip()).build();
        let node2 = runtime.create_node().ip(addr2.ip()).build();
        let id1 = node1.id();
        let id2 = node2.id();
        let barrier = Arc::new(Barrier::new(2));
        let barrier_ = barrier.clone();

        let f1 = node1.spawn(async move {
            // phase1
            let net = plugin::simulator::<NetSim>();
            net.clog_node(id1);
            let listener = TcpListener::bind(addr1).await.unwrap();
            barrier.wait().await;

            // phase2
            timeout(Duration::from_secs(1), listener.accept())
                .await
                .expect_err("listener should not get connection");
            barrier.wait().await;

            // phase3
            net.unclog_node(id1);
            barrier.wait().await;
            let (mut stream, _) = listener.accept().await.unwrap();
            stream.write_all(b"hello world").await.unwrap();
            stream.flush().await.unwrap();
            barrier.wait().await;

            // phase4
            net.clog_link(id1, id2);
            net.clog_link(id2, id1);
            crate::task::spawn(async move {
                crate::time::sleep(Duration::from_secs(5)).await;
                net.unclog_link(id1, id2);
                net.unclog_link(id2, id1);
            });
            barrier.wait().await;
            stream.write_all(b"hello world").await.unwrap();
            stream.flush().await.unwrap();

            stream
        });

        let f2 = node2.spawn(async move {
            // phase1
            barrier_.wait().await;

            // phase2
            TcpStream::connect(addr1)
                .await
                .expect_err("connect should fail");
            barrier_.wait().await;

            // phase3
            barrier_.wait().await;
            let mut stream = TcpStream::connect(addr1).await.unwrap();
            let mut buf = [0; 20];
            let len = stream.read(&mut buf).await.unwrap();
            assert_eq!(&buf[0..len], b"hello world");
            barrier_.wait().await;

            // phase4
            barrier_.wait().await;
            let len = stream.read(&mut buf).await.unwrap();
            assert_eq!(&buf[0..len], b"hello world");

            stream
        });

        runtime.block_on(f1).unwrap();
        runtime.block_on(f2).unwrap();
    }

    #[test]
    fn reset() {
        let runtime = Runtime::new();
        let addr1 = "10.0.0.1:1".parse::<SocketAddr>().unwrap();
        let addr2 = "10.0.0.2:1".parse::<SocketAddr>().unwrap();
        let node1 = runtime.create_node().ip(addr1.ip()).build();
        let node2 = runtime.create_node().ip(addr2.ip()).build();
        let barrier = Arc::new(Barrier::new(2));
        let barrier_ = barrier.clone();

        node1.spawn(async move {
            let listener = TcpListener::bind(addr1).await.unwrap();
            barrier.wait().await;
            let (_stream, _) = listener.accept().await.unwrap();
            barrier.wait().await;
            std::future::pending::<()>().await;
        });

        let f2 = node2.spawn(async move {
            barrier_.wait().await;
            let mut stream = TcpStream::connect(addr1).await.unwrap();
            barrier_.wait().await;

            let net = plugin::simulator::<NetSim>();
            net.reset_node(node1.id());

            let mut buf = [0; 20];
            let len = stream.read(&mut buf).await.expect("read should return EOF");
            assert_eq!(len, 0);
        });

        runtime.block_on(f2).unwrap();
    }

    #[test]
    fn ip_resolve() {
        let runtime = Runtime::new();
        let node1 = runtime
            .create_node()
            .ip("10.0.0.1".parse().unwrap())
            .build();

        let f1 = node1.spawn(async move {
            assert_eq!(
                TcpListener::bind("10.0.0.2:10000")
                    .await
                    .unwrap_err()
                    .kind(),
                ErrorKind::AddrNotAvailable,
            );

            let _l1 = TcpListener::bind("10.0.0.1:10000").await.unwrap();
            assert_eq!(
                TcpStream::connect("127.0.0.1:10000")
                    .await
                    .unwrap_err()
                    .kind(),
                ErrorKind::ConnectionRefused,
            );
            assert_eq!(
                TcpStream::connect("0.0.0.0:10000")
                    .await
                    .unwrap_err()
                    .kind(),
                ErrorKind::ConnectionRefused
            );

            let _l2 = TcpListener::bind("0.0.0.0:10000").await.unwrap();
            TcpStream::connect("0.0.0.0:10000").await.unwrap();

            let _l3 = TcpListener::bind("127.0.0.1:10000").await.unwrap();
            TcpStream::connect("127.0.0.1:10000").await.unwrap();
        });
        runtime.block_on(f1).unwrap();
    }

    #[test]
    fn ipvs_load_balance() {
        let runtime = Runtime::new();
        let addr1 = "10.0.0.1:1".parse::<SocketAddr>().unwrap();
        let addr2 = "10.0.0.2:1".parse::<SocketAddr>().unwrap();
        let addr3 = "10.0.0.3:1".parse::<SocketAddr>().unwrap();
        let node1 = runtime.create_node().ip(addr1.ip()).build();
        let node2 = runtime.create_node().ip(addr2.ip()).build();
        let node3 = runtime.create_node().ip(addr3.ip()).build();

        // set virtual service for load balance
        runtime.block_on(async {
            let net = NetSim::current();
            let ipvs = net.global_ipvs();
            ipvs.add_service(ServiceAddr::Tcp("1.1.1.1:80".into()), Scheduler::RoundRobin);
            ipvs.add_server(ServiceAddr::Tcp("1.1.1.1:80".into()), "10.0.0.1:1");
            ipvs.add_server(ServiceAddr::Tcp("1.1.1.1:80".into()), "10.0.0.2:1");
        });

        let barrier = Arc::new(Barrier::new(3));
        let barrier1 = barrier.clone();
        let barrier2 = barrier.clone();
        let f1 = node1.spawn(async move {
            let listener = TcpListener::bind("0.0.0.0:1").await.unwrap();
            barrier1.wait().await;
            let (mut stream, _) = listener.accept().await.unwrap();

            let mut buf = [0; 20];
            let len = stream.read(&mut buf).await.unwrap();
            assert_eq!(&buf[0..len], b"1");
        });
        let f2 = node2.spawn(async move {
            let listener = TcpListener::bind("0.0.0.0:1").await.unwrap();
            barrier2.wait().await;
            let (mut stream, _) = listener.accept().await.unwrap();

            let mut buf = [0; 20];
            let len = stream.read(&mut buf).await.unwrap();
            assert_eq!(&buf[0..len], b"2");
        });
        let f3 = node3.spawn(async move {
            barrier.wait().await;
            let mut stream1 = TcpStream::connect("1.1.1.1:80").await.unwrap(); // go to node1
            let mut stream2 = TcpStream::connect("1.1.1.1:80").await.unwrap(); // go to node2
            stream1.write_all(b"1").await.unwrap();
            stream1.flush().await.unwrap();
            stream2.write_all(b"2").await.unwrap();
            stream2.flush().await.unwrap();
        });
        runtime.block_on(async move {
            f1.await.unwrap();
            f2.await.unwrap();
            f3.await.unwrap();
        });
    }
}
