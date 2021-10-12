//! Asynchronous network endpoint and a controlled network simulator.
//!
//! # Examples
//!
//! ```
//! # use madsim_std as madsim;
//! use madsim::{Runtime, net::NetLocalHandle};
//!
//! let runtime = Runtime::new();
//! let host1 = runtime.create_host("127.0.0.1:0").unwrap();
//! let host2 = runtime.create_host("127.0.0.1:0").unwrap();
//! let addr1 = host1.local_addr();
//! let addr2 = host2.local_addr();
//!
//! host1
//!     .spawn(async move {
//!         let net = NetLocalHandle::current();
//!         net.send_to(addr2, 1, &[1]).await.unwrap();
//!     })
//!     .detach();
//!
//! let f = host2.spawn(async move {
//!     let net = NetLocalHandle::current();
//!     let mut buf = vec![0; 0x10];
//!     let (len, from) = net.recv_from(1, &mut buf).await.unwrap();
//!     assert_eq!(from, addr1);
//!     assert_eq!(&buf[..len], &[1]);
//! });
//!
//! runtime.block_on(f);
//! ```

pub use self::network::{Config, Stat};
#[cfg(not(feature = "ucx"))]
pub use self::tcp::*;
#[cfg(feature = "ucx")]
pub use self::ucx::*;

mod network;
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
        let host1 = rt.create_host("127.0.0.1:0").unwrap();
        let host2 = rt.create_host("127.0.0.1:0").unwrap();
        let addr1 = host1.local_addr();
        let addr2 = host2.local_addr();

        host1
            .spawn(async move {
                let net = NetLocalHandle::current();
                net.send_to(addr2, 1, &[1]).await.unwrap();

                sleep(Duration::from_millis(10)).await;
                net.send_to(addr2, 2, &[2]).await.unwrap();
            })
            .detach();

        let f = host2.spawn(async move {
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
