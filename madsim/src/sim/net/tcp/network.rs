use std::{
    collections::{HashMap, HashSet, VecDeque}, 
    net::{SocketAddr}, 
    sync::{atomic::{AtomicU64, Ordering}, Mutex, Arc}, task::{Waker, Context}, time::Duration, 
};

use futures::{channel::mpsc, SinkExt};
use log::{debug, trace};
use rand::Rng;

use crate::{task::NodeId, rand::GlobalRng, time::TimeHandle};

use super::{Payload, Config};


/// an inner simulated implementation of Tcp
/// which contains all the tcp network nodes in the simulation system.
pub(crate) struct TcpNetwork {
    inner: Arc<Mutex<Inner>>
}

struct Inner {
    rand: GlobalRng,
    time: TimeHandle,
    config: Config,
    accept: HashMap<SocketAddr, (NodeId, mpsc::Sender<(ConnId, ConnId)>)>,
    conn: HashMap<ConnId, Connection>,
    clogged_node: HashSet<NodeId>,
    clogged_link: HashSet<(NodeId, NodeId)>,
    conn_cnt: AtomicU64
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
                conn_cnt: 0.into()
            }))
        }
    }

    pub fn update_config(&self, f: impl FnOnce(&mut Config)) {
        let mut inner = self.inner.lock().unwrap();
        f(&mut inner.config);
    }

    pub(crate) fn clog_node(&self, id: NodeId) {
        debug!("tcp clog: {id}");
        self.inner.lock().unwrap().clogged_node.insert(id);
    }

    pub(crate) fn unclog_node(&self, id: NodeId) {
        debug!("tcp unclog: {id}");
        self.inner.lock().unwrap().clogged_node.remove(&id);
    }

    pub(crate) fn clog_link(&self, src: NodeId, dst: NodeId) {
        debug!("clog: {src} -> {dst}");
        self.inner.lock().unwrap().clogged_link.insert((src, dst));
    }

    pub(crate) fn unclog_link(&self, src: NodeId, dst: NodeId) {
        debug!("tcp unclog: {src} -> {dst}");
        self.inner.lock().unwrap().clogged_link.remove(&(src, dst));
    }

    pub(crate) fn listen(&self, id: NodeId, addr: SocketAddr) -> Result<mpsc::Receiver<(ConnId, ConnId)>, String> {
        let (tx, rx) = mpsc::channel(0);
        match self.inner.lock().unwrap().accept.insert(addr, (id, tx)) {
            Some(_) => Err("addr is in used".to_string()),
            None => Ok(rx)
        }
    }

    pub async fn connect(&self, src: NodeId, dst: &SocketAddr) -> Result<(ConnId, ConnId), String> {
        let (send_conn, recv_conn, mut tx) = {
            let mut inner = self.inner.lock().unwrap();

            let (dst, tx) = if let Some((id, tx)) = inner.accept.get_mut(dst) {
                (*id, tx.clone())
            } else {
                return Err("addr not be listened".to_string());
            };

            let send_conn = inner.conn_cnt.fetch_add(1, Ordering::SeqCst);
            let recv_conn = inner.conn_cnt.fetch_add(1, Ordering::SeqCst);
        
            inner.conn.insert(send_conn, Connection::new(src, dst));
            inner.conn.insert(recv_conn, Connection::new(dst, src));

            debug!("tcp connect to conn({}, {})", send_conn, recv_conn);
            (send_conn, recv_conn, tx)
        };
        

        // the other part has the reflex conn
        // if move this send call site into above block, the mutex will cross await
        tx.send((recv_conn, send_conn)).await.map_err(|e| e.to_string())?;

        debug!("tcp connect conn({}, {})", send_conn, recv_conn);
        Ok((send_conn, recv_conn))
    }

    pub fn send(&self, id: &ConnId, msg: Payload) -> Result<usize, String> {
        
        let (mut rand, time, config) = {
            let mut inner = self.inner.lock().unwrap();

            let (src, dst) = if let Some(conn) = inner.conn.get_mut(id) {
                    conn.is_block = true;
                    (conn.src, conn.dst)
            } else {
                return Err("conn closed".to_string());
            };
    
            if inner.clogged_node.contains(&src)
                || inner.clogged_node.contains(&dst)
                || inner.clogged_link.contains(&(src, dst))
            {
                trace!("clogged");
                return Err("clogged".to_string());
            }

            (inner.rand.clone(), inner.time.clone(), inner.config.clone())
        };

        let mut latency = Duration::default();
        let mut cnt = 1;
        // simulated timout resend
        while rand.gen_bool(config.packet_loss_rate) {
            cnt += 1;
        }

        latency += rand.gen_range(config.send_latency.clone()) * cnt;

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


    pub fn recv(&self, id: &ConnId, cx: Option<&mut Context<'_>>) -> Result<Option<Payload>, String> {
        match self.inner.lock().unwrap().conn.get_mut(id) {
            Some(conn) => {
                if conn.is_block || conn.data.is_empty() {
                    trace!("wait for tcp msg at {:?}", conn);
                    match cx {
                        Some(cx) => conn.wakers.push_back(cx.waker().clone()),
                        None => ()
                    }
                    Ok(None)
                } else {
                    let msg = conn.data.pop_front();
                    trace!("tcp get msg: {:?}", msg);
                    Ok(msg)
                }
            }
            None => Err("conn closed".to_string())
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
            dst 
        }
    } 
}

pub type ConnId = u64;
