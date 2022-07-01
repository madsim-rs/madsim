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
    fn receiver_drop() {
        let runtime = Runtime::new();
        let addr1 = "10.0.0.1:1".parse::<SocketAddr>().unwrap();
        let addr2 = "10.0.0.2:1".parse::<SocketAddr>().unwrap();
        let node1 = runtime.create_node().ip(addr1.ip()).build();
        let node2 = runtime.create_node().ip(addr2.ip()).build();
        let barrier = Arc::new(Barrier::new(2));
        let barrier_ = barrier.clone();

        let barrier2 = Arc::new(Barrier::new(2));
        let barrier2_ = barrier2.clone();

        let f = node1.spawn(async move {
            let listener = TcpListener::bind(addr1).await.unwrap();
            barrier_.wait().await;
            let (mut stream, _) = listener.accept().await.unwrap();
            let buf = [104, 101, 108, 108, 111, 32, 119, 111, 114, 108, 100];
            barrier2.wait().await;
            stream.write(&buf).await.unwrap();
        });

        node2.spawn(async move {
            barrier.wait().await;
            let mut stream = TcpStream::connect(addr1).await.unwrap();
            let mut buf = [0; 20];
            timeout(Duration::from_secs(1), stream.read(&mut buf))
                .await
                .err()
                .unwrap();
            barrier2_.wait().await;
            let n = stream.read(&mut buf).await.unwrap();
            assert_eq!(
                &buf[0..n],
                [104, 101, 108, 108, 111, 32, 119, 111, 114, 108, 100]
            );
        });

        runtime.block_on(f).unwrap();
    }

    #[test]
    fn connect_disconnect() {
        let runtime = Runtime::new();
        let addr1 = "10.0.0.1:1".parse::<SocketAddr>().unwrap();
        let addr2 = "10.0.0.2:1".parse::<SocketAddr>().unwrap();
        let node1 = runtime.create_node().ip(addr1.ip()).build();
        let node2 = runtime.create_node().ip(addr2.ip()).build();
        let id1 = node1.id();
        let id2 = node2.id();
        let barrier = Arc::new(Barrier::new(2));
        let barrier_ = barrier.clone();

        let sim = runtime.block_on(async move {
            let sim = plugin::simulator::<TcpSim>();
            sim.disconnect(id1);
            sim
        });

        let f1 = node1.spawn(async move {
            let listener = TcpListener::bind(addr1).await.unwrap();
            barrier.wait().await;
            let (mut stream, _) = listener.accept().await.unwrap();
            let buf = [104, 101, 108, 108, 111, 32, 119, 111, 114, 108, 100];
            stream.write(&buf).await.err().unwrap();
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

        let mut stream1 = runtime.block_on(f1).unwrap();
        let mut stream2 = runtime.block_on(f2).unwrap();
        let sim = runtime.block_on(async move {
            sim.connect(id1);
            sim
        });

        let f1 = node1.spawn(async move {
            let buf = [104, 101, 108, 108, 111, 32, 119, 111, 114, 108, 100];
            stream1.write(&buf).await.unwrap();
            stream1
        });

        let f2 = node2.spawn(async move {
            let mut buf = [0; 20];
            let n = stream2.read(&mut buf).await.unwrap();
            assert_eq!(
                &buf[0..n],
                [104, 101, 108, 108, 111, 32, 119, 111, 114, 108, 100]
            );
            stream2
        });
        let mut stream1 = runtime.block_on(f1).unwrap();
        let mut stream2 = runtime.block_on(f2).unwrap();
        let sim = runtime.block_on(async move {
            sim.disconnect2(id1, id2);
            sim
        });

        let f1 = node1.spawn(async move {
            let buf = [104, 101, 108, 108, 111, 32, 119, 111, 114, 108, 100];
            stream1.write(&buf).await.err().unwrap();
            stream1
        });

        let f2 = node2.spawn(async move {
            let mut buf = [0; 20];
            timeout(Duration::from_secs(1), stream2.read(&mut buf))
                .await
                .err()
                .unwrap();
            stream2
        });

        let mut stream1 = runtime.block_on(f1).unwrap();
        let mut stream2 = runtime.block_on(f2).unwrap();
        let _ = runtime.block_on(async move {
            sim.connect2(id1, id2);
            sim
        });

        let f1 = node1.spawn(async move {
            let buf = [104, 101, 108, 108, 111, 32, 119, 111, 114, 108, 100];
            stream1.write(&buf).await.unwrap();
            stream1
        });

        let f2 = node2.spawn(async move {
            let mut buf = [0; 20];
            let n = stream2.read(&mut buf).await.unwrap();
            assert_eq!(
                &buf[0..n],
                [104, 101, 108, 108, 111, 32, 119, 111, 114, 108, 100]
            );
            stream2
        });

        let _ = runtime.block_on(f1).unwrap();
        let _ = runtime.block_on(f2).unwrap();
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
        let barrier2 = Arc::new(Barrier::new(3));
        let barrier2_ = barrier2.clone();
        let barrier2__ = barrier2.clone();
        let barrier3 = Arc::new(Barrier::new(3));
        let barrier3_ = barrier3.clone();
        let barrier3__ = barrier3.clone();

        let f1 = node1.spawn(async move {
            let listener = TcpListener::bind(addr1).await.unwrap();
            barrier.wait().await;
            let (mut stream, _) = listener.accept().await.unwrap();
            barrier2.wait().await;
            barrier3.wait().await;

            let buf = [104, 101, 108, 108, 111, 32, 119, 111, 114, 108, 100];
            stream.write(&buf).await.err().unwrap();
        });

        let f2 = node2.spawn(async move {
            barrier_.wait().await;
            let mut stream = TcpStream::connect(addr1).await.unwrap();
            barrier2_.wait().await;
            barrier3_.wait().await;

            let mut buf = [0; 20];
            timeout(Duration::from_secs(1), stream.read(&mut buf))
                .await
                .unwrap()
                .err()
                .unwrap();
        });

        runtime.block_on(async move {
            barrier__.wait().await;
            barrier2__.wait().await;
            plugin::simulator::<TcpSim>().reset_node(node1.id());
            barrier3__.wait().await;

            f1.await.unwrap();
            f2.await.unwrap();
        });
    }
}
