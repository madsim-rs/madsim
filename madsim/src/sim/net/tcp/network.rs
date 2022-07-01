use std::{
    collections::{hash_map::Entry, HashMap, HashSet, VecDeque},
    io,
    net::SocketAddr,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex,
    },
    task::{Context, Waker},
    time::Duration,
};

use log::trace;
use rand::Rng;

use crate::{rand::GlobalRng, task::NodeId, time::TimeHandle, Config};

use super::Payload;

/// an inner simulated implementation of Tcp
/// which contains all the tcp network nodes in the simulation system.
pub(crate) struct TcpNetwork {
    inner: Arc<Mutex<Inner>>,
}

pub(crate) struct Inner {
    rand: GlobalRng,
    time: TimeHandle,
    config: Config,
    accept: HashMap<SocketAddr, (NodeId, async_channel::Sender<(ConnId, ConnId)>)>,
    conn: HashMap<ConnId, Connection>,
    clogged_node: HashSet<NodeId>,
    clogged_link: HashSet<(NodeId, NodeId)>,
    conn_cnt: AtomicU64,
}

impl TcpNetwork {
    pub(crate) fn new(rand: GlobalRng, time: TimeHandle, config: Config) -> Self {
        Self {
            inner: Arc::new(Mutex::new(Inner {
                rand,
                time,
                config,
                accept: HashMap::default(),
                conn: HashMap::default(),
                clogged_node: HashSet::default(),
                clogged_link: HashSet::default(),
                conn_cnt: 0.into(),
            })),
        }
    }

    pub(crate) fn update_config(&self, f: impl FnOnce(&mut Config)) {
        let mut inner = self.inner.lock().unwrap();
        f(&mut inner.config);
    }

    pub(crate) fn reset_node(&self, node: NodeId) -> io::Result<()> {
        trace!("reset node {}", node);
        let mut inner = self.inner.lock().unwrap();
        let mut drop_ids = vec![];

        for (conn_id, conn) in &inner.conn {
            if conn.src == node || conn.dst == node {
                drop_ids.push(*conn_id);
            }
        }

        if drop_ids.is_empty() {
            Err(io::Error::new(
                io::ErrorKind::NotFound,
                "there is no node to reset".to_string(),
            ))
        } else {
            for id in drop_ids {
                inner.conn.remove(&id);
            }

            Ok(())
        }
    }

    pub(crate) fn drop_connection(&self, conn: ConnId) -> io::Result<()> {
        let mut inner = self.inner.lock().unwrap();
        let removed = inner.conn.remove(&conn).is_some();
        if removed {
            Ok(())
        } else {
            Err(io::Error::new(
                io::ErrorKind::NotConnected,
                "connection not exist or already shutdown".to_string(),
            ))
        }
    }

    pub(crate) fn clog_node(&self, id: NodeId) {
        trace!("tcp clog: {id}");
        self.inner.lock().unwrap().clogged_node.insert(id);
    }

    pub(crate) fn unclog_node(&self, id: NodeId) {
        trace!("tcp unclog: {id}");
        self.inner.lock().unwrap().clogged_node.remove(&id);
    }

    pub(crate) fn clog_link(&self, src: NodeId, dst: NodeId) {
        trace!("clog: {src} -> {dst}");
        self.inner.lock().unwrap().clogged_link.insert((src, dst));
    }

    pub(crate) fn unclog_link(&self, src: NodeId, dst: NodeId) {
        trace!("tcp unclog: {src} -> {dst}");
        self.inner.lock().unwrap().clogged_link.remove(&(src, dst));
    }

    pub(crate) fn listen(
        &self,
        id: NodeId,
        addr: SocketAddr,
    ) -> io::Result<async_channel::Receiver<(ConnId, ConnId)>> {
        let (tx, rx) = async_channel::unbounded();
        let mut inner = self.inner.lock().unwrap();
        let entry = inner.accept.entry(addr);

        match entry {
            Entry::Occupied(_) => Err(io::Error::new(
                io::ErrorKind::AddrInUse,
                "addr is in used".to_string(),
            )),
            Entry::Vacant(o) => {
                o.insert((id, tx));
                Ok(rx)
            }
        }
    }

    pub(crate) async fn connect(
        &self,
        src: NodeId,
        dst: &SocketAddr,
    ) -> io::Result<(ConnId, ConnId)> {
        let (send_conn, recv_conn, tx) = {
            let mut inner = self.inner.lock().unwrap();

            let (dst, tx) = if let Some((id, tx)) = inner.accept.get_mut(dst) {
                (*id, tx.clone())
            } else {
                return Err(io::Error::new(
                    io::ErrorKind::ConnectionRefused,
                    "ConnectionRefused, maybe the server not listened at".to_string(),
                ));
            };

            let send_conn = inner.conn_cnt.fetch_add(2, Ordering::SeqCst);
            let recv_conn = send_conn + 1;

            inner.conn.insert(send_conn, Connection::new(src, dst));
            inner.conn.insert(recv_conn, Connection::new(dst, src));

            trace!("tcp connect to conn({}, {})", send_conn, recv_conn);
            (send_conn, recv_conn, tx)
        };

        // the other part has the reflex conn
        // if move this send call site into above block, the mutex will cross await
        tx.send((recv_conn, send_conn))
            .await
            .map_err(|e| io::Error::new(io::ErrorKind::Interrupted, e.to_string()))?;

        trace!("tcp connect conn({}, {})", send_conn, recv_conn);
        Ok((send_conn, recv_conn))
    }

    pub(crate) fn send(&self, id: &ConnId, msg: Payload) -> io::Result<usize> {
        let (mut rand, time, config) = {
            let mut inner = self.inner.lock().unwrap();

            let (src, dst) = if let Some(conn) = inner.conn.get_mut(id) {
                conn.is_block = true;
                (conn.src, conn.dst)
            } else {
                return Err(io::Error::new(
                    io::ErrorKind::ConnectionReset,
                    "connection closed".to_string(),
                ));
            };

            if inner.clogged_node.contains(&src)
                || inner.clogged_node.contains(&dst)
                || inner.clogged_link.contains(&(src, dst))
            {
                trace!("clogged");
                let msg = msg.downcast::<Vec<u8>>().unwrap();
                let n = msg.len();
                return Ok(n);
            }

            (inner.rand.clone(), inner.time.clone(), inner.config.clone())
        };

        let mut latency = Duration::default();
        let mut cnt = 1;
        // simulated timout resend
        while rand.gen_bool(config.net.packet_loss_rate) {
            cnt += 1;
        }

        latency += rand.gen_range(config.net.send_latency) * cnt;

        let msg = msg.downcast::<Vec<u8>>().unwrap();
        let n = msg.len();
        trace!("delay: {latency:?}");

        let inner_ = self.inner.clone();
        let id_ = *id;

        time.add_timer(time.now() + latency, move || {
            let mut inner = inner_.lock().unwrap();
            let conn = inner.conn.get_mut(&id_).unwrap();
            conn.data.push_back(msg);
            conn.is_block = false;
            while let Some(waker) = conn.wakers.pop_front() {
                waker.wake();
            }
        });

        Ok(n)
    }

    pub(crate) fn recv(
        &self,
        id: &ConnId,
        cx: Option<&mut Context<'_>>,
    ) -> io::Result<Option<Payload>> {
        let mut inner = self.inner.lock().unwrap();
        let conn = inner.conn.get_mut(id);
        match conn {
            Some(conn) => {
                if conn.is_block || conn.data.is_empty() {
                    trace!("wait for tcp msg at {:?}", conn);
                    match cx {
                        Some(cx) => conn.wakers.push_back(cx.waker().clone()),
                        None => (),
                    }
                    Ok(None)
                } else {
                    let msg = conn.data.pop_front();
                    trace!("tcp get msg: {:?}", msg);
                    Ok(msg)
                }
            }
            None => Err(io::Error::new(
                io::ErrorKind::ConnectionReset,
                "connection closed".to_string(),
            )),
        }
    }
}

#[derive(Debug)]
pub(crate) struct Connection {
    data: VecDeque<Payload>,
    wakers: VecDeque<Waker>,
    is_block: bool,
    src: NodeId,
    dst: NodeId,
}

impl Connection {
    pub(crate) fn new(src: NodeId, dst: NodeId) -> Self {
        Self {
            data: VecDeque::default(),
            wakers: VecDeque::default(),
            is_block: false,
            src,
            dst,
        }
    }
}

pub type ConnId = u64;
