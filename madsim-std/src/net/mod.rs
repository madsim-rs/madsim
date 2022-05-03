//! Asynchronous network endpoint and a controlled network simulator.
//!
//! # Examples
//!
//! ```
//! # use madsim_std as madsim;
//! use madsim::{Runtime, net::Endpoint};
//! use std::net::SocketAddr;
//!
//! let runtime = Runtime::new();
//! let node = runtime.create_node().build().unwrap();
//! let (tx1, rx1) = tokio::sync::oneshot::channel::<SocketAddr>();
//! let (tx2, rx2) = tokio::sync::oneshot::channel::<SocketAddr>();
//!
//! node
//!     .spawn(async move {
//!         let ep = Endpoint::bind("127.0.0.1:0").await.unwrap();
//!         tx1.send(ep.local_addr().unwrap()).unwrap();
//!         let addr2 = rx2.await.unwrap();
//!
//!         ep.send_to(addr2, 1, &[1]).await.unwrap();
//!     })
//!     .detach();
//!
//! runtime.block_on(async move {
//!     let ep = Endpoint::bind("127.0.0.1:0").await.unwrap();
//!     tx2.send(ep.local_addr().unwrap()).unwrap();
//!     let addr1 = rx1.await.unwrap();
//!
//!     let mut buf = vec![0; 0x10];
//!     let (len, from) = ep.recv_from(1, &mut buf).await.unwrap();
//!     assert_eq!(from, addr1);
//!     assert_eq!(&buf[..len], &[1]);
//! });
//! ```

#[cfg(not(feature = "ucx"))]
pub use self::tcp::*;
#[cfg(feature = "ucx")]
pub use self::ucx::*;

pub use std::net::SocketAddr;

#[cfg(feature = "rpc")]
pub mod rpc;
#[cfg(not(feature = "ucx"))]
mod tcp;
#[cfg(feature = "ucx")]
mod ucx;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{time::*, Runtime};

    #[test]
    fn send_recv() {
        let rt = Runtime::new();
        let node1 = rt.create_node().build().unwrap();
        let node2 = rt.create_node().build().unwrap();
        let (tx1, rx1) = tokio::sync::oneshot::channel::<SocketAddr>();
        let (tx2, rx2) = tokio::sync::oneshot::channel::<SocketAddr>();

        node1
            .spawn(async move {
                let ep = Endpoint::bind("127.0.0.1:0").await.unwrap();
                tx1.send(ep.local_addr().unwrap()).unwrap();
                let addr2 = rx2.await.unwrap();

                ep.send_to(addr2, 1, &[1]).await.unwrap();

                sleep(Duration::from_millis(10)).await;
                ep.send_to(addr2, 2, &[2]).await.unwrap();
            })
            .detach();

        let f = node2.spawn(async move {
            let ep = Endpoint::bind("127.0.0.1:0").await.unwrap();
            tx2.send(ep.local_addr().unwrap()).unwrap();
            let addr1 = rx1.await.unwrap();

            let mut buf = vec![0; 0x10];
            let (len, from) = ep.recv_from(2, &mut buf).await.unwrap();
            assert_eq!(len, 1);
            assert_eq!(from, addr1);
            assert_eq!(buf[0], 2);

            let (len, from) = ep.recv_from(1, &mut buf).await.unwrap();
            assert_eq!(len, 1);
            assert_eq!(from, addr1);
            assert_eq!(buf[0], 1);
        });

        rt.block_on(f);
    }
}
