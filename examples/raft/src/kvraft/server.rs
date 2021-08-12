use super::msg::*;
use crate::raft;
use futures::{channel::oneshot, StreamExt};
use madsim::{
    fs, net,
    rand::{self, Rng},
    task,
    time::{timeout, Duration},
};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fmt,
    net::SocketAddr,
    sync::{Arc, Mutex},
};

pub struct KvServer {
    rf: raft::RaftHandle,
    me: usize,
    // { index -> (id, sender) }
    pending_rpcs: Arc<Mutex<HashMap<u64, (u64, oneshot::Sender<String>)>>>,
    _bg_task: task::Task<()>,
}

impl fmt::Debug for KvServer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "KvServer({})", self.me)
    }
}

impl KvServer {
    pub async fn new(
        servers: Vec<SocketAddr>,
        me: usize,
        max_raft_state: Option<usize>,
    ) -> Arc<KvServer> {
        // You may need initialization code here.
        let (rf, mut apply_ch) = raft::RaftHandle::new(servers, me).await;

        let pending_rpcs = Arc::new(Mutex::new(
            HashMap::<u64, (u64, oneshot::Sender<String>)>::new(),
        ));
        let pending_rpcs0 = pending_rpcs.clone();
        let rf0 = rf.clone();
        let _bg_task = task::spawn(async move {
            let mut state = State::default();
            let mut state_index;
            while let Some(msg) = apply_ch.next().await {
                match msg {
                    raft::ApplyMsg::Snapshot { index, data, .. } => {
                        state = flexbuffers::from_slice(&data).unwrap();
                        state_index = index;
                    }
                    raft::ApplyMsg::Command { index, data } => {
                        let cmd: RaftCmd = flexbuffers::from_slice(&data).unwrap();
                        let cmd_id = cmd.id;
                        let ret = state.apply(cmd);
                        state_index = index;

                        // send result to RPC
                        let mut pending_rpcs = pending_rpcs0.lock().unwrap();
                        if let Some((id, sender)) = pending_rpcs.remove(&index) {
                            if cmd_id == id {
                                // message match, success
                                let _ = sender.send(ret);
                            }
                            // otherwise drop the sender
                        }
                    }
                }
                // snapshot if needed
                if let Some(size) = max_raft_state {
                    if fs::metadata("state").await.map(|m| m.len()).unwrap_or(0) >= size as u64 {
                        let data = flexbuffers::to_vec(&state).unwrap();
                        rf0.snapshot(state_index, &data).await.unwrap();
                    }
                }
            }
        });

        let this = Arc::new(KvServer {
            rf,
            me,
            pending_rpcs,
            _bg_task,
        });
        this.start_rpc_server();
        this
    }

    fn start_rpc_server(self: &Arc<Self>) {
        let net = net::NetworkLocalHandle::current();

        let this = self.clone();
        net.add_rpc_handler(move |args: PutAppendArgs| {
            let this = this.clone();
            async move { this.put_append(args).await }
        });

        let this = self.clone();
        net.add_rpc_handler(move |args: GetArgs| {
            let this = this.clone();
            async move { this.get(args).await }
        });
    }

    fn register_rpc(&self, index: u64, id: u64) -> oneshot::Receiver<String> {
        let (sender, recver) = oneshot::channel();
        self.pending_rpcs
            .lock()
            .unwrap()
            .insert(index, (id, sender));
        recver
    }

    /// The current term of this peer.
    pub fn term(&self) -> u64 {
        self.rf.term()
    }

    /// Whether this peer believes it is the leader.
    pub fn is_leader(&self) -> bool {
        self.rf.is_leader()
    }

    async fn get(&self, arg: GetArgs) -> GetReply {
        // Your code here.
        let cmd = RaftCmd {
            id: rand::rng().gen(),
            op: Op::Get {
                key: arg.key.clone(),
            },
        };
        let index = match self.rf.start(&flexbuffers::to_vec(&cmd).unwrap()).await {
            Ok(s) => s.index,
            Err(raft::Error::NotLeader) => return GetReply::WrongLeader,
            e => panic!("{:?}", e),
        };
        let recver = self.register_rpc(index, cmd.id);
        debug!("{:?} start {:?}", self, arg);
        match timeout(Duration::from_millis(500), recver).await {
            Err(_) => GetReply::Timeout,
            Ok(Err(_)) => GetReply::Failed,
            Ok(Ok(value)) => GetReply::Ok { value },
        }
    }

    async fn put_append(&self, arg: PutAppendArgs) -> PutAppendReply {
        // Your code here.
        let cmd = RaftCmd {
            id: arg.id,
            op: match arg.append {
                false => Op::Put {
                    key: arg.key.clone(),
                    value: arg.value.clone(),
                },
                true => Op::Append {
                    key: arg.key.clone(),
                    value: arg.value.clone(),
                },
            },
        };
        let index = match self.rf.start(&flexbuffers::to_vec(&cmd).unwrap()).await {
            Ok(s) => s.index,
            Err(raft::Error::NotLeader) => return PutAppendReply::WrongLeader,
            e => panic!("{:?}", e),
        };
        let recver = self.register_rpc(index, cmd.id);
        debug!("{:?} start {:?}", self, arg);
        match timeout(Duration::from_millis(500), recver).await {
            Err(_) => PutAppendReply::Timeout,
            Ok(Err(_)) => PutAppendReply::Failed,
            Ok(Ok(_)) => PutAppendReply::Ok,
        }
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct State {
    kv: HashMap<String, String>,
    // A circular queue with max capacity 50
    ids: Vec<u64>,
}

impl State {
    fn apply(&mut self, cmd: RaftCmd) -> String {
        let unique = !self.ids.contains(&cmd.id);
        if self.ids.len() > 50 {
            self.ids.remove(0);
        }
        self.ids.push(cmd.id);
        match cmd.op {
            Op::Put { key, value } => {
                // prevent duplicate put
                if unique {
                    self.kv.insert(key, value);
                }
                "".into()
            }
            Op::Append { key, value } => {
                // prevent duplicate append
                if unique {
                    self.kv.entry(key).or_default().push_str(&value);
                }
                "".into()
            }
            Op::Get { key } => self.kv.get(&key).cloned().unwrap_or_default(),
        }
    }
}
