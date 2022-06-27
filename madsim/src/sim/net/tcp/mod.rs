//!
use std::{any::Any};

///
pub type Payload = Box<dyn Any + Send + Sync>;

mod network;
pub (crate) mod sim;
mod listener;
pub use listener::*;
mod stream;
pub use stream::*;
mod addr;
pub(crate) use addr::to_socket_addrs;
pub use addr::ToSocketAddrs;

#[cfg(test)]
mod tests {
    use std::{net::SocketAddr, sync::Arc};

    use super::*;
    use crate::runtime::Runtime;
    use tokio::sync::Barrier;
    use tokio::io::{AsyncWriteExt, AsyncReadExt};

    #[test]
    fn send_recv() {
        let runtime = Runtime::new();
        let addr1 = "10.0.0.1:1".parse::<SocketAddr>().unwrap();
        let addr2 = "10.0.0.2:1".parse::<SocketAddr>().unwrap();
        let node1 = runtime.create_node().ip(addr1.ip()).build();
        let node2 = runtime.create_node().ip(addr2.ip()).build();
        let barrier = Arc::new(Barrier::new(2));

        let barrier_ = barrier.clone();
        node1.spawn(async move {
            let mut listener = TcpListener::bind(addr1).await.unwrap();
            barrier_.wait().await;
            let (mut stream, _) = listener.accept().await.unwrap();
            let buf = "hello world".to_string();
            stream.write(buf.as_bytes()).await.unwrap();
        });

        let f = node2.spawn(async move {
            barrier.wait().await;
            let mut stream = TcpStream::connect(addr1).await.unwrap();
            let mut buf = [0; 10];
            let n = stream.read(&mut buf).await.unwrap();
            println!("recv: {:?}", &buf[0..n]);
        });

        runtime.block_on(f).unwrap();


    }
}
