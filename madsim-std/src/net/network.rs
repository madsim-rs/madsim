use bytes::Bytes;
use std::{net::SocketAddr, ops::Range, time::Duration};
use tokio::sync::oneshot;

/// Network configurations.
#[derive(Debug, Default)]
#[allow(missing_docs)]
pub struct Config {
    pub packet_loss_rate: f64,
    pub send_latency: Range<Duration>,
}

/// Network statistics.
#[derive(Debug, Default, Clone)]
pub struct Stat {
    /// Total number of messages.
    pub msg_count: u64,
}

#[derive(Debug)]
pub struct RecvMsg {
    pub tag: u64,
    pub data: Bytes,
    pub from: SocketAddr,
}

#[derive(Default)]
pub struct Mailbox {
    registered: Vec<(u64, oneshot::Sender<RecvMsg>)>,
    msgs: Vec<RecvMsg>,
}

impl Mailbox {
    pub fn send(&mut self, msg: RecvMsg) {
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

    pub fn recv(&mut self, tag: u64) -> oneshot::Receiver<RecvMsg> {
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
