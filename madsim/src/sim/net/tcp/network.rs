use std::{
    collections::{HashMap, HashSet, VecDeque}, 
    net::{SocketAddr}, 
    sync::{atomic::{AtomicU64, Ordering}, Mutex}, task::{Waker, Context}, 
};

use futures::{channel::mpsc, SinkExt};
use log::debug;

use crate::{task::NodeId};

use super::Payload;


/// an inner simulated implementation of Tcp
/// which contains all the tcp network nodes in the simulation system.
pub(crate) struct TcpNetwork {
    inner: Mutex<Inner>
}

#[derive(Default)]
struct Inner {
    accept: HashMap<SocketAddr, (NodeId, mpsc::Sender<(ConnId, ConnId)>)>,
    conn: HashMap<ConnId, Connection>,
    clogged_node: HashSet<NodeId>,
    clogged_link: HashSet<(NodeId, NodeId)>,
    conn_cnt: AtomicU64
}



impl TcpNetwork {
    pub(crate) fn new() -> Self {
        Self {
            inner: Mutex::new(Inner::default())
        }
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

            // let (send_conn, recv_conn, dst) = match  {
            let send_conn = inner.conn_cnt.fetch_add(1, Ordering::SeqCst);
            let recv_conn = inner.conn_cnt.fetch_add(1, Ordering::SeqCst);
        
            inner.conn.insert(send_conn, Connection::new(src, dst));
            inner.conn.insert(recv_conn, Connection::new(dst, src));
            (send_conn, recv_conn, tx)
        };
        

        // the other part has the reflex conn
        // if move this send call site into above block, the mutex will cross await
        tx.send((recv_conn, send_conn)).await.map_err(|e| e.to_string())?;

        debug!("tcp connect conn({}, {})", send_conn, recv_conn);
        Ok((send_conn, recv_conn))
    }

    pub fn send(&self, id: &ConnId, msg: Payload) -> Result<usize, String> {
        match self.inner.lock().unwrap().conn.get_mut(id) {
            Some(conn) => {
                let msg = msg.downcast::<Vec<u8>>().unwrap();
                let n = msg.len();
                debug!("tcp send to {}, msg: {:?}", id, msg);
                conn.data.push_back(msg);
                if let Some(waker) = conn.wakers.pop_front() {
                    waker.wake();
                }
                Ok(n)
            }
            None => Err("conn closed".to_string())
        }
    }


    pub fn recv(&self, id: &ConnId, cx: Option<&mut Context<'_>>) -> Result<Option<Payload>, String> {
        match self.inner.lock().unwrap().conn.get_mut(id) {
            Some(conn) => {
                if conn.data.is_empty() {
                    debug!("wait for tcp msg");
                    match cx {
                        Some(cx) => conn.wakers.push_back(cx.waker().clone()),
                        None => ()
                    }
                    Ok(None)
                } else {
                    let msg = conn.data.pop_front();
                    println!("tcp get msg: {:?}", msg);
                    Ok(msg)
                }
            }
            None => Err("conn closed".to_string())
        }
    }

}

pub(crate) struct Connection {
    data: VecDeque<Payload>,
    wakers: VecDeque<Waker>,
    src: NodeId,
    dst: NodeId,
}

impl Connection {
    pub(crate) fn new(src: NodeId, dst: NodeId) -> Self {
        Self { data: VecDeque::default(), wakers: VecDeque::default(), src, dst }
    } 
}

pub type ConnId = u64;

#[cfg(test)]
mod tests {
    
}