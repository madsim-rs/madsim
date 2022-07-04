//! a tokio Tcp Simulatior
//!
//! # Examples
//!
//! ```
//!
//! use madsim::{net::{TcpSim, TcpStream, TcpListener}, plugin, runtime::Runtime, time::timeout};
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
//!     let buf = [104, 101, 108, 108, 111, 32, 119, 111, 114, 108, 100];
//!     stream.write(&buf).await.unwrap();
//! });
//!
//! let f2 = node2.spawn(async move {
//!     barrier.wait().await;
//!     let mut stream = TcpStream::connect(addr1).await.unwrap();
//!     let mut buf = [0; 20];
//!     let n = stream.read(&mut buf).await.unwrap();
//!     assert_eq!(
//!         &buf[0..n],
//!         [104, 101, 108, 108, 111, 32, 119, 111, 114, 108, 100]
//!     );
//! });
//!
//! runtime.block_on(f1).unwrap();
//! runtime.block_on(f2).unwrap();
//! ```

use std::any::Any;

/// tcp packet payload
pub type Payload = Box<dyn Any + Send + Sync>;

mod listener;
mod network;
pub(crate) mod sim;
pub use listener::*;
mod stream;
pub use stream::*;
mod addr;
pub(crate) use addr::to_socket_addrs;
pub use addr::ToSocketAddrs;

mod config;
pub use config::*;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{net::TcpSim, plugin, runtime::Runtime, time::timeout};
    use std::{net::SocketAddr, sync::Arc, time::Duration};
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
            let buf = [104, 101, 108, 108, 111, 32, 119, 111, 114, 108, 100];
            stream.write(&buf).await.unwrap();
        });

        let f2 = node2.spawn(async move {
            barrier.wait().await;
            let mut stream = TcpStream::connect(addr1).await.unwrap();
            let mut buf = [0; 20];
            let n = stream.read(&mut buf).await.unwrap();
            assert_eq!(
                &buf[0..n],
                [104, 101, 108, 108, 111, 32, 119, 111, 114, 108, 100]
            );
        });

        runtime.block_on(f1).unwrap();
        runtime.block_on(f2).unwrap();
    }

    #[test]
    fn disconnect() {
        let runtime = Runtime::new();
        let addr1 = "10.0.0.1:1".parse::<SocketAddr>().unwrap();
        let addr2 = "10.0.0.2:1".parse::<SocketAddr>().unwrap();
        let node1 = runtime.create_node().ip(addr1.ip()).build();
        let node2 = runtime.create_node().ip(addr2.ip()).build();
        let id1 = node1.id();
        let barrier = Arc::new(Barrier::new(2));
        let barrier_ = barrier.clone();

        runtime.block_on(async move {
            let sim = plugin::simulator::<TcpSim>();
            sim.disconnect(id1);
            sim
        });

        let f1 = node1.spawn(async move {
            let listener = TcpListener::bind(addr1).await.unwrap();
            barrier.wait().await;
            let (mut stream, _) = listener.accept().await.unwrap();
            let buf = [104, 101, 108, 108, 111, 32, 119, 111, 114, 108, 100];
            stream.write(&buf).await.unwrap();
            stream
        });

        let f2 = node2.spawn(async move {
            barrier_.wait().await;
            let mut stream = TcpStream::connect(addr1).await.unwrap();
            let mut buf = [0; 20];
            timeout(Duration::from_secs(1), stream.read(&mut buf))
                .await
                .err()
                .unwrap();
            stream
        });

        runtime.block_on(f1).unwrap();
        runtime.block_on(f2).unwrap();
    }

    #[test]
    fn recovery() {
        let runtime = Runtime::new();
        let addr1 = "10.0.0.1:1".parse::<SocketAddr>().unwrap();
        let addr2 = "10.0.0.2:1".parse::<SocketAddr>().unwrap();
        let node1 = runtime.create_node().ip(addr1.ip()).build();
        let node2 = runtime.create_node().ip(addr2.ip()).build();
        let id1 = node1.id();
        let barrier = Arc::new(Barrier::new(2));
        let barrier_ = barrier.clone();

        runtime.block_on(async move {
            let sim = plugin::simulator::<TcpSim>();
            sim.disconnect(id1);
            sim.connect(id1);
        });

        let f1 = node1.spawn(async move {
            let listener = TcpListener::bind(addr1).await.unwrap();
            barrier.wait().await;
            let (mut stream, _) = listener.accept().await.unwrap();
            let buf = [104, 101, 108, 108, 111, 32, 119, 111, 114, 108, 100];
            stream.write(&buf).await.unwrap();
        });

        let f2 = node2.spawn(async move {
            barrier_.wait().await;
            let mut stream = TcpStream::connect(addr1).await.unwrap();
            let mut buf = [0; 20];
            let n = stream.read(&mut buf).await.unwrap();
            assert_eq!(
                &buf[0..n],
                [104, 101, 108, 108, 111, 32, 119, 111, 114, 108, 100]
            );
        });

        runtime.block_on(f1).unwrap();
        runtime.block_on(f2).unwrap();
    }

    #[test]
    fn disconnect2() {
        let runtime = Runtime::new();
        let addr1 = "10.0.0.1:1".parse::<SocketAddr>().unwrap();
        let addr2 = "10.0.0.2:1".parse::<SocketAddr>().unwrap();
        let node1 = runtime.create_node().ip(addr1.ip()).build();
        let node2 = runtime.create_node().ip(addr2.ip()).build();
        let id1 = node1.id();
        let id2 = node2.id();
        let barrier = Arc::new(Barrier::new(2));
        let barrier_ = barrier.clone();

        runtime.block_on(async move {
            let sim = plugin::simulator::<TcpSim>();
            sim.disconnect2(id1, id2);
        });

        let f1 = node1.spawn(async move {
            let listener = TcpListener::bind(addr1).await.unwrap();
            barrier.wait().await;
            let (mut stream, _) = listener.accept().await.unwrap();
            let buf = [104, 101, 108, 108, 111, 32, 119, 111, 114, 108, 100];
            stream.write(&buf).await.unwrap();
        });

        let f2 = node2.spawn(async move {
            barrier_.wait().await;
            let mut stream = TcpStream::connect(addr1).await.unwrap();
            let mut buf = [0; 20];
            timeout(Duration::from_secs(1), stream.read(&mut buf))
                .await
                .err()
                .unwrap();
        });

        runtime.block_on(f1).unwrap();
        runtime.block_on(f2).unwrap();
    }

    #[test]
    fn recovery2() {
        let runtime = Runtime::new();
        let addr1 = "10.0.0.1:1".parse::<SocketAddr>().unwrap();
        let addr2 = "10.0.0.2:1".parse::<SocketAddr>().unwrap();
        let node1 = runtime.create_node().ip(addr1.ip()).build();
        let node2 = runtime.create_node().ip(addr2.ip()).build();
        let id1 = node1.id();
        let id2 = node2.id();
        let barrier = Arc::new(Barrier::new(2));
        let barrier_ = barrier.clone();

        runtime.block_on(async move {
            let sim = plugin::simulator::<TcpSim>();
            sim.disconnect2(id1, id2);
            sim.connect2(id1, id2);
        });

        let f1 = node1.spawn(async move {
            let listener = TcpListener::bind(addr1).await.unwrap();
            barrier.wait().await;
            let (mut stream, _) = listener.accept().await.unwrap();
            let buf = [104, 101, 108, 108, 111, 32, 119, 111, 114, 108, 100];
            stream.write(&buf).await.unwrap();
        });

        let f2 = node2.spawn(async move {
            barrier_.wait().await;
            let mut stream = TcpStream::connect(addr1).await.unwrap();
            let mut buf = [0; 20];
            let n = stream.read(&mut buf).await.unwrap();
            assert_eq!(
                &buf[0..n],
                [104, 101, 108, 108, 111, 32, 119, 111, 114, 108, 100]
            );
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
        let barrier = Arc::new(Barrier::new(3));
        let barrier_ = barrier.clone();
        let barrier__ = barrier.clone();

        let f1 = node1.spawn(async move {
            let listener = TcpListener::bind(addr1).await.unwrap();
            barrier.wait().await;
            let (mut stream, _) = listener.accept().await.unwrap();
            barrier.wait().await;
            barrier.wait().await;

            let buf = [104, 101, 108, 108, 111, 32, 119, 111, 114, 108, 100];
            stream.write(&buf).await.err().unwrap();
        });

        let f2 = node2.spawn(async move {
            barrier_.wait().await;
            let mut stream = TcpStream::connect(addr1).await.unwrap();
            barrier_.wait().await;
            barrier_.wait().await;

            let mut buf = [0; 20];
            timeout(Duration::from_secs(1), stream.read(&mut buf))
                .await
                .unwrap()
                .err()
                .unwrap();
        });

        runtime.block_on(async move {
            barrier__.wait().await;
            barrier__.wait().await;
            plugin::simulator::<TcpSim>().reset_node(node1.id());
            barrier__.wait().await;

            f1.await.unwrap();
            f2.await.unwrap();
        });
    }
}
