//! Asynchronous network endpoint and a controlled network simulator.
//!
//! # Examples
//!
//! ```
//! # use madsim_std as madsim;
//! use madsim::{Runtime, net::NetLocalHandle};
//!
//! let runtime = Runtime::new();
//! let node1 = runtime.create_node("127.0.0.1:0").build().unwrap();
//! let node2 = runtime.create_node("127.0.0.1:0").build().unwrap();
//! let addr1 = node1.local_addr();
//! let addr2 = node2.local_addr();
//!
//! node1
//!     .spawn(async move {
//!         let net = NetLocalHandle::current();
//!         net.send_to(addr2, 1, &[1]).await.unwrap();
//!     })
//!     .detach();
//!
//! let f = node2.spawn(async move {
//!     let net = NetLocalHandle::current();
//!     let mut buf = vec![0; 0x10];
//!     let (len, from) = net.recv_from(1, &mut buf).await.unwrap();
//!     assert_eq!(from, addr1);
//!     assert_eq!(&buf[..len], &[1]);
//! });
//!
//! runtime.block_on(f);
//! ```

#[cfg(not(feature = "ucx"))]
pub use self::tcp::*;
#[cfg(feature = "ucx")]
pub use self::ucx::*;

#[cfg(feature = "rpc")]
pub mod rpc;
#[cfg(not(feature = "ucx"))]
mod tcp;
#[cfg(feature = "ucx")]
mod ucx;

#[cfg(test)]
mod tests {
    use super::NetLocalHandle;
    use crate::{time::*, Runtime};

    #[test]
    fn send_recv() {
        let rt = Runtime::new();
        let node1 = rt.create_node("127.0.0.1:0").build().unwrap();
        let node2 = rt.create_node("127.0.0.1:0").build().unwrap();
        let addr1 = node1.local_addr();
        let addr2 = node2.local_addr();

        node1
            .spawn(async move {
                let net = NetLocalHandle::current();
                net.send_to(addr2, 1, &[1]).await.unwrap();

                sleep(Duration::from_millis(10)).await;
                net.send_to(addr2, 2, &[2]).await.unwrap();
            })
            .detach();

        let f = node2.spawn(async move {
            let net = NetLocalHandle::current();
            let mut buf = vec![0; 0x10];
            let (len, from) = net.recv_from(2, &mut buf).await.unwrap();
            assert_eq!(len, 1);
            assert_eq!(from, addr1);
            assert_eq!(buf[0], 2);
            let (len, from) = net.recv_from(1, &mut buf).await.unwrap();
            assert_eq!(len, 1);
            assert_eq!(from, addr1);
            assert_eq!(buf[0], 1);
        });

        rt.block_on(f);
    }
}
