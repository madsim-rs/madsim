//! Asynchronous network endpoint and a controlled network simulator.
//!
//! # Examples
//!
//! ```
//! use madsim::net::Endpoint;
//! use std::net::SocketAddr;
//!
//! let runtime = tokio::runtime::Runtime::new().unwrap();
//! let (tx1, rx1) = tokio::sync::oneshot::channel::<SocketAddr>();
//! let (tx2, rx2) = tokio::sync::oneshot::channel::<SocketAddr>();
//!
//! runtime.spawn(async move {
//!     let ep = Endpoint::bind("127.0.0.1:0").await.unwrap();
//!     tx1.send(ep.local_addr().unwrap()).unwrap();
//!     let addr2 = rx2.await.unwrap();
//!
//!     ep.send_to(addr2, 1, &[1]).await.unwrap();
//! });
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

#[cfg(feature = "rpc")]
#[cfg_attr(docsrs, doc(cfg(feature = "rpc")))]
pub mod rpc;
#[cfg(not(feature = "ucx"))]
mod tcp;
#[cfg(feature = "ucx")]
mod ucx;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::time::*;
    use std::net::SocketAddr;

    #[tokio::test]
    async fn send_recv() {
        let (tx1, rx1) = tokio::sync::oneshot::channel::<SocketAddr>();
        let (tx2, rx2) = tokio::sync::oneshot::channel::<SocketAddr>();

        tokio::spawn(async move {
            let ep = Endpoint::bind("127.0.0.1:0").await.unwrap();
            tx1.send(ep.local_addr().unwrap()).unwrap();
            let addr2 = rx2.await.unwrap();

            ep.send_to(addr2, 1, &[1]).await.unwrap();

            sleep(Duration::from_millis(10)).await;
            ep.send_to(addr2, 2, &[2]).await.unwrap();
        });

        tokio::spawn(async move {
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
        })
        .await
        .unwrap();
    }
}
