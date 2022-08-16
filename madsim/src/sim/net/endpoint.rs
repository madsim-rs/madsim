use super::{IpProtocol::Udp, *};

/// An endpoint.
pub struct Endpoint {
    guard: BindGuard,
    socket: Arc<EndpointSocket>,
    pub(super) peer: Mutex<Option<SocketAddr>>,
    /// Incoming connections.
    conn_rx: async_channel::Receiver<(Sender, Receiver)>,
}

impl Endpoint {
    /// Creates a [`Endpoint`] from the given address.
    pub async fn bind(addr: impl ToSocketAddrs) -> io::Result<Self> {
        let (conn_tx, conn_rx) = async_channel::unbounded();
        let socket = Arc::new(EndpointSocket {
            mailbox: Mutex::new(Mailbox::default()),
            conn_tx,
            node: plugin::node(),
        });
        let guard = BindGuard::bind(addr, Udp, socket.clone()).await?;
        Ok(Endpoint {
            guard,
            socket,
            peer: Mutex::new(None),
            conn_rx,
        })
    }

    /// Connects this [`Endpoint`] to a remote address.
    pub async fn connect(addr: impl ToSocketAddrs) -> io::Result<Self> {
        let peer = lookup_host(addr).await?.next().unwrap();
        let ep = Self::bind("0.0.0.0:0").await?;
        *ep.peer.lock() = Some(peer);
        Ok(ep)
    }

    /// Returns the local socket address.
    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        Ok(self.guard.addr)
    }

    /// Returns the socket address of the remote peer this socket was connected to.
    pub fn peer_addr(&self) -> io::Result<SocketAddr> {
        (self.peer.lock())
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotConnected, "not connected"))
    }

    /// Sends data with tag on the socket to the given address.
    ///
    /// # Example
    /// ```
    /// use madsim::{runtime::Runtime, net::Endpoint};
    ///
    /// Runtime::new().block_on(async {
    ///     let net = Endpoint::bind("127.0.0.1:0").await.unwrap();
    ///     net.send_to("127.0.0.1:4242", 0, &[0; 10]).await.expect("couldn't send data");
    /// });
    /// ```
    pub async fn send_to(&self, dst: impl ToSocketAddrs, tag: u64, buf: &[u8]) -> io::Result<()> {
        let dst = lookup_host(dst).await?.next().unwrap();
        self.send_to_raw(dst, tag, Box::new(Vec::from(buf))).await
    }

    /// Receives a single message with given tag on the socket.
    /// On success, returns the number of bytes read and the origin.
    ///
    /// # Example
    /// ```no_run
    /// use madsim::{runtime::Runtime, net::Endpoint};
    ///
    /// Runtime::new().block_on(async {
    ///     let net = Endpoint::bind("127.0.0.1:0").await.unwrap();
    ///     let mut buf = [0; 10];
    ///     let (len, src) = net.recv_from(0, &mut buf).await.expect("couldn't receive data");
    /// });
    /// ```
    pub async fn recv_from(&self, tag: u64, buf: &mut [u8]) -> io::Result<(usize, SocketAddr)> {
        let (data, from) = self.recv_from_raw(tag).await?;
        // copy to buffer
        let data = data.downcast::<Vec<u8>>().expect("message is not data");
        let len = buf.len().min(data.len());
        buf[..len].copy_from_slice(&data[..len]);
        Ok((len, from))
    }

    /// Sends data on the socket to the remote address to which it is connected.
    pub async fn send(&self, tag: u64, buf: &[u8]) -> io::Result<()> {
        let peer = self.peer_addr()?;
        self.send_to(peer, tag, buf).await
    }

    /// Receives a single datagram message on the socket from the remote address to which it is connected.
    /// On success, returns the number of bytes read.
    pub async fn recv(&self, tag: u64, buf: &mut [u8]) -> io::Result<usize> {
        let peer = self.peer_addr()?;
        let (len, from) = self.recv_from(tag, buf).await?;
        assert_eq!(
            from, peer,
            "receive a message but not from the connected address"
        );
        Ok(len)
    }

    /// Sends a raw message.
    ///
    /// NOTE: Applications should not use this function!
    /// It is provided for use by other simulators.
    #[cfg_attr(docsrs, doc(cfg(madsim)))]
    pub async fn send_to_raw(&self, dst: SocketAddr, tag: u64, data: Payload) -> io::Result<()> {
        trace!("send: {} -> {dst}, tag={tag}", self.guard.addr);
        self.guard
            .net
            .send(
                self.guard.node,
                self.guard.addr.port(),
                dst,
                Udp,
                Box::new((tag, data)),
            )
            .await?;
        Ok(())
    }

    /// Receives a raw message.
    ///
    /// NOTE: Applications should not use this function!
    /// It is provided for use by other simulators.
    #[cfg_attr(docsrs, doc(cfg(madsim)))]
    pub async fn recv_from_raw(&self, tag: u64) -> io::Result<(Payload, SocketAddr)> {
        let recver = self.socket.mailbox.lock().recv(tag);
        let msg = recver
            .await
            .map_err(|_| io::Error::new(io::ErrorKind::BrokenPipe, "network is down"))?;
        self.guard.net.rand_delay().await?;

        trace!("recv: {} <- {}, tag={}", self.guard.addr, msg.from, msg.tag);
        Ok((msg.data, msg.from))
    }

    /// Sends a raw message. to the connected remote address.
    ///
    /// NOTE: Applications should not use this function!
    /// It is provided for use by other simulators.
    #[cfg_attr(docsrs, doc(cfg(madsim)))]
    pub async fn send_raw(&self, tag: u64, data: Payload) -> io::Result<()> {
        let peer = self.peer_addr()?;
        self.send_to_raw(peer, tag, data).await
    }

    /// Receives a raw message from the connected remote address.
    ///
    /// NOTE: Applications should not use this function!
    /// It is provided for use by other simulators.
    #[cfg_attr(docsrs, doc(cfg(madsim)))]
    pub async fn recv_raw(&self, tag: u64) -> io::Result<Payload> {
        let peer = self.peer_addr()?;
        let (msg, from) = self.recv_from_raw(tag).await?;
        assert_eq!(
            from, peer,
            "receive a message but not from the connected address"
        );
        Ok(msg)
    }

    /// Setup a reliable connection to another Endpoint.
    #[doc(hidden)]
    pub async fn connect1(&self, addr: SocketAddr) -> io::Result<(Sender, Receiver)> {
        self.guard.net.rand_delay().await?;

        let (conn_tx, conn_rx) = oneshot::channel::<(Sender, Receiver)>();
        self.guard
            .net
            .send(
                self.guard.node,
                self.guard.addr.port(),
                addr,
                Udp,
                Box::new(conn_tx),
            )
            .await?;
        let (tx, rx) = conn_rx
            .await
            .map_err(|e| io::Error::new(io::ErrorKind::ConnectionRefused, e))?;
        Ok((tx, rx))
    }

    /// Accept a new connection from other Endpoint.
    #[doc(hidden)]
    pub async fn accept1(&self) -> io::Result<(Sender, Receiver)> {
        self.guard.net.rand_delay().await?;

        (self.conn_rx.recv().await).map_err(|e| io::Error::new(io::ErrorKind::ConnectionReset, e))
    }
}

#[doc(hidden)]
pub struct Sender {
    tx: mpsc::UnboundedSender<Payload>,
}

#[doc(hidden)]
pub struct Receiver {
    rx: mpsc::UnboundedReceiver<Payload>,
}

impl Sender {
    #[doc(hidden)]
    pub async fn send(&self, value: Payload) -> io::Result<()> {
        (self.tx.send(value))
            .map_err(|_| io::Error::new(io::ErrorKind::ConnectionReset, "connection reset"))
    }
}

impl Receiver {
    #[doc(hidden)]
    pub async fn recv(&mut self) -> io::Result<Payload> {
        (self.rx.recv().await).ok_or(io::Error::new(
            io::ErrorKind::ConnectionReset,
            "connection reset",
        ))
    }
}

struct Message {
    tag: u64,
    data: Payload,
    from: SocketAddr,
}

type Payload = Box<dyn Any + Send + Sync>;

/// Tag message mailbox for an endpoint.
#[derive(Default)]
struct Mailbox {
    /// Pending receive requests.
    registered: Vec<(u64, oneshot::Sender<Message>)>,
    /// Messages that have not been received.
    msgs: Vec<Message>,
}

struct EndpointSocket {
    mailbox: Mutex<Mailbox>,
    conn_tx: async_channel::Sender<(Sender, Receiver)>,
    node: NodeId,
}

impl Socket for EndpointSocket {
    fn deliver(&self, src: SocketAddr, dst: SocketAddr, msg: Payload) {
        // tag send
        let msg = match msg.downcast::<(u64, Payload)>() {
            Ok(msg) => {
                let (tag, data) = *msg;
                self.mailbox.lock().deliver(Message {
                    tag,
                    data,
                    from: src,
                });
                return;
            }
            Err(msg) => msg,
        };
        // connection
        let (src_node, tx): (NodeId, oneshot::Sender<(Sender, Receiver)>) =
            *msg.downcast().unwrap();
        let net = plugin::simulator::<NetSim>();
        let (tx1, rx1) = net.channel(src_node, dst, Udp);
        let (tx2, rx2) = net.channel(self.node, src, Udp);
        let _ = tx.send((Sender { tx: tx1 }, Receiver { rx: rx2 }));
        let _ = self
            .conn_tx
            .try_send((Sender { tx: tx2 }, Receiver { rx: rx1 }));
    }
}

impl Mailbox {
    fn deliver(&mut self, msg: Message) {
        let mut i = 0;
        let mut msg = Some(msg);
        while i < self.registered.len() {
            if matches!(&msg, Some(msg) if msg.tag == self.registered[i].0) {
                // tag match, take and try send
                let (_, sender) = self.registered.swap_remove(i);
                msg = match sender.send(msg.take().unwrap()) {
                    Ok(_) => return,
                    Err(m) => Some(m),
                };
                // failed to send, try next
            } else {
                // tag mismatch, move to next
                i += 1;
            }
        }
        // failed to match awaiting recv, save
        self.msgs.push(msg.unwrap());
    }

    fn recv(&mut self, tag: u64) -> oneshot::Receiver<Message> {
        let (tx, rx) = oneshot::channel();
        if let Some(idx) = self.msgs.iter().position(|msg| tag == msg.tag) {
            let msg = self.msgs.swap_remove(idx);
            tx.send(msg).ok().unwrap();
        } else {
            self.registered.push((tag, tx));
        }
        rx
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{plugin::simulator, runtime::Runtime, time::*};
    use tokio::sync::Barrier;

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
            let net = Endpoint::bind(addr1).await.unwrap();
            barrier_.wait().await;

            net.send_to(addr2, 1, &[1]).await.unwrap();

            sleep(Duration::from_secs(1)).await;
            net.send_to(addr2, 2, &[2]).await.unwrap();
        });

        let f = node2.spawn(async move {
            let net = Endpoint::bind(addr2).await.unwrap();
            barrier.wait().await;

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

        runtime.block_on(f).unwrap();
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
        node1.spawn(async move {
            let net = Endpoint::bind(addr1).await.unwrap();
            barrier_.wait().await;

            net.send_to(addr2, 1, &[1]).await.unwrap();
        });

        let f = node2.spawn(async move {
            let net = Endpoint::bind(addr2).await.unwrap();
            let mut buf = vec![0; 0x10];
            timeout(Duration::from_secs(1), net.recv_from(1, &mut buf))
                .await
                .err()
                .unwrap();
            // timeout and receiver dropped here
            barrier.wait().await;

            // receive again should success
            let (len, from) = net.recv_from(1, &mut buf).await.unwrap();
            assert_eq!(len, 1);
            assert_eq!(from, addr1);
        });

        runtime.block_on(f).unwrap();
    }

    #[test]
    #[ignore] // TODO: rethink what happens when network "resets"
    fn reset() {
        let runtime = Runtime::new();
        let addr1 = "10.0.0.1:1".parse::<SocketAddr>().unwrap();
        let node1 = runtime.create_node().ip(addr1.ip()).build();

        let f = node1.spawn(async move {
            let net = Endpoint::bind(addr1).await.unwrap();
            let err = net.recv_from(1, &mut []).await.unwrap_err();
            assert_eq!(err.kind(), std::io::ErrorKind::BrokenPipe);
            // FIXME: should still error
            // let err = net.recv_from(1, &mut []).await.unwrap_err();
            // assert_eq!(err.kind(), std::io::ErrorKind::BrokenPipe);
        });

        runtime.block_on(async move {
            sleep(Duration::from_secs(1)).await;
            simulator::<NetSim>().reset_node(node1.id());
            f.await.unwrap();
        });
    }

    #[test]
    fn bind() {
        let runtime = Runtime::new();
        let ip = "10.0.0.1".parse::<IpAddr>().unwrap();
        let node = runtime.create_node().ip(ip).build();

        let f = node.spawn(async move {
            // unspecified
            let ep = Endpoint::bind("0.0.0.0:0").await.unwrap();
            let addr = ep.local_addr().unwrap();
            assert_eq!(addr.ip().to_string(), "0.0.0.0");
            assert_ne!(addr.port(), 0);

            // unspecified v6
            let ep = Endpoint::bind(":::0").await.unwrap();
            let addr = ep.local_addr().unwrap();
            assert_eq!(addr.ip().to_string(), "::");
            assert_ne!(addr.port(), 0);

            // localhost
            let ep = Endpoint::bind("127.0.0.1:0").await.unwrap();
            let addr = ep.local_addr().unwrap();
            assert_eq!(addr.ip().to_string(), "127.0.0.1");
            assert_ne!(addr.port(), 0);

            // localhost v6
            let ep = Endpoint::bind("::1:0").await.unwrap();
            let addr = ep.local_addr().unwrap();
            assert_eq!(addr.ip().to_string(), "::1");
            assert_ne!(addr.port(), 0);

            // wrong IP
            let err = Endpoint::bind("10.0.0.2:0").await.err().unwrap();
            assert_eq!(err.kind(), std::io::ErrorKind::AddrNotAvailable);

            // local IP
            let ep = Endpoint::bind("10.0.0.1:100").await.unwrap();
            assert_eq!(ep.local_addr().unwrap().to_string(), "10.0.0.1:100");

            // drop and reuse port
            drop(ep);
            let _ = Endpoint::bind("10.0.0.1:100").await.unwrap();
        });
        runtime.block_on(f).unwrap();
    }

    #[test]
    fn localhost() {
        let runtime = Runtime::new();
        let ip1 = "10.0.0.1".parse::<IpAddr>().unwrap();
        let ip2 = "10.0.0.2".parse::<IpAddr>().unwrap();
        let node1 = runtime.create_node().ip(ip1).build();
        let node2 = runtime.create_node().ip(ip2).build();
        let barrier = Arc::new(Barrier::new(2));

        let barrier_ = barrier.clone();
        let f1 = node1.spawn(async move {
            let ep1 = Endpoint::bind("127.0.0.1:1").await.unwrap();
            let ep2 = Endpoint::bind("10.0.0.1:2").await.unwrap();
            barrier_.wait().await;

            // FIXME: ep1 should not receive messages from other node
            timeout(Duration::from_secs(1), ep1.recv_from(1, &mut []))
                .await
                .err()
                .expect("localhost endpoint should not receive from other nodes");
            // ep2 should receive
            let (_, from) = ep2.recv_from(1, &mut []).await.unwrap();
            assert_eq!(from.to_string(), "10.0.0.2:1");
        });
        let f2 = node2.spawn(async move {
            let ep = Endpoint::bind("127.0.0.1:1").await.unwrap();
            barrier.wait().await;

            ep.send_to("10.0.0.1:1", 1, &[1]).await.unwrap();
            ep.send_to("10.0.0.1:2", 1, &[1]).await.unwrap();
        });
        runtime.block_on(f1).unwrap();
        runtime.block_on(f2).unwrap();
    }

    #[test]
    fn connect_send_recv() {
        let runtime = Runtime::new();
        let addr1 = "10.0.0.1:1".parse::<SocketAddr>().unwrap();
        let addr2 = "10.0.0.2:1".parse::<SocketAddr>().unwrap();
        let node1 = runtime.create_node().ip(addr1.ip()).build();
        let node2 = runtime.create_node().ip(addr2.ip()).build();
        let barrier = Arc::new(Barrier::new(2));

        let barrier_ = barrier.clone();
        node1.spawn(async move {
            let ep = Endpoint::bind(addr1).await.unwrap();
            assert_eq!(ep.local_addr().unwrap(), addr1);
            barrier_.wait().await;

            let mut buf = vec![0; 0x10];
            let (len, from) = ep.recv_from(1, &mut buf).await.unwrap();
            assert_eq!(&buf[..len], b"ping");

            ep.send_to(from, 1, b"pong").await.unwrap();
        });

        let f = node2.spawn(async move {
            barrier.wait().await;
            let ep = Endpoint::connect(addr1).await.unwrap();
            assert_eq!(ep.peer_addr().unwrap(), addr1);

            ep.send(1, b"ping").await.unwrap();

            let mut buf = vec![0; 0x10];
            let len = ep.recv(1, &mut buf).await.unwrap();
            assert_eq!(&buf[..len], b"pong");
        });

        runtime.block_on(f).unwrap();
    }
}
