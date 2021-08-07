use crate::raft::*;
use futures::StreamExt;
use log::*;
use madsim::{
    rand::{self, Rng},
    time::{self, Instant},
    Handle,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::{Arc, Mutex},
    time::Duration,
};

pub struct RaftTester {
    handle: Handle,
    n: usize,
    addrs: Vec<SocketAddr>,
    rafts: Vec<RaftHandle>,
    connected: Vec<bool>,
    storage: StorageHandle,
    // stat
    t0: Instant,
    rpc0: u64,
}

pub const SNAPSHOT_INTERVAL: u64 = 10;

impl RaftTester {
    pub async fn new(n: usize) -> Self {
        let handle = Handle::current();
        let mut tester = RaftTester {
            n,
            addrs: (0..n)
                .map(|i| SocketAddr::from(([0, 0, 0, i as _], 0)))
                .collect::<Vec<_>>(),
            rafts: vec![], // construct later
            connected: vec![false; n],
            storage: StorageHandle::new(n),
            t0: Instant::now(),
            rpc0: 0,
            handle,
        };
        tester.rpc0 = tester.rpc_total();
        for i in 0..n {
            let raft = tester.start1_ext(i, false).await;
            tester.rafts.push(raft);
        }
        tester
    }

    /// Check that there's exactly one leader.
    /// Try a few times in case re-elections are needed.
    pub async fn check_one_leader(&self) -> usize {
        let mut random = rand::rng();
        let mut leaders = HashMap::<u64, Vec<usize>>::new();
        for _iters in 0..10 {
            let ms = 450 + (random.gen::<u64>() % 100);
            time::sleep(Duration::from_millis(ms)).await;

            for (i, connected) in self.connected.iter().enumerate() {
                if !connected {
                    continue;
                }
                let raft = &self.rafts[i];
                if raft.is_leader() {
                    leaders.entry(raft.term()).or_default().push(i);
                }
            }
            for (&term, leaders) in &leaders {
                if leaders.len() > 1 {
                    panic!("term {} has {:?} (>1) leaders", term, leaders);
                }
            }
            if !leaders.is_empty() {
                let last_term_with_leader = leaders.keys().max().unwrap();
                return leaders[&last_term_with_leader][0];
            }
        }
        panic!("expected one leader, got none")
    }

    /// Check that everyone agrees on the term.
    pub fn check_terms(&self) -> u64 {
        let mut term = 0;
        for (i, connected) in self.connected.iter().enumerate() {
            if !connected {
                continue;
            }
            let xterm = self.rafts[i].term();
            if term == 0 {
                term = xterm;
            } else if term != xterm {
                panic!("servers disagree on term");
            }
        }
        term
    }

    /// Check that there's no leader
    pub fn check_no_leader(&self) {
        for (i, connected) in self.connected.iter().enumerate() {
            if !connected {
                continue;
            }
            if self.rafts[i].is_leader() {
                panic!("expected no leader, but {} claims to be leader", i);
            }
        }
    }

    pub fn term(&self, i: usize) -> u64 {
        self.rafts[i].term()
    }

    pub fn rpc_total(&self) -> u64 {
        self.handle.net.stat().msg_count
    }

    /// How many servers think a log entry is committed?
    pub fn n_committed(&self, index: u64) -> (usize, Option<Entry>) {
        self.storage.n_committed(index)
    }

    pub fn start(&self, i: usize, cmd: Entry) -> Result<Start, Error> {
        self.rafts[i].start(&flexbuffers::to_vec(&cmd).unwrap())
    }

    /// wait for at least n servers to commit.
    /// but don't wait forever.
    pub async fn wait(&self, index: u64, n: usize, start_term: Option<u64>) -> Option<Entry> {
        let mut to = Duration::from_millis(10);
        for _ in 0..30 {
            let (nd, _) = self.n_committed(index);
            if nd >= n {
                break;
            }
            time::sleep(to).await;
            if to < Duration::from_secs(1) {
                to *= 2;
            }
            if let Some(start_term) = start_term {
                for rf in self.rafts.iter() {
                    if rf.term() > start_term {
                        // someone has moved on
                        // can no longer guarantee that we'll "win"
                        return None;
                    }
                }
            }
        }
        let (nd, cmd) = self.n_committed(index);
        if nd < n {
            panic!("only {} decided for index {}; wanted {}", nd, index, n);
        }
        cmd
    }

    /// Do a complete agreement.
    ///
    /// it might choose the wrong leader initially,
    /// and have to re-submit after giving up.
    /// entirely gives up after about 10 seconds.
    /// indirectly checks that the servers agree on the
    /// same value, since n_committed() checks this,
    /// as do the threads that read from applyCh.
    /// returns index.
    /// if retry==true, may submit the command multiple
    /// times, in case a leader fails just after Start().
    /// if retry==false, calls start() only once, in order
    /// to simplify the early Lab 2B tests.
    pub async fn one(&self, cmd: Entry, expected_servers: usize, retry: bool) -> u64 {
        let t0 = Instant::now();
        let mut starts = 0;
        while t0.elapsed() < Duration::from_secs(10) {
            // try all the servers, maybe one is the leader.
            let mut index = None;
            for _ in 0..self.n {
                starts = (starts + 1) % self.n;
                if !self.connected[starts] {
                    continue;
                }
                let rf = &self.rafts[starts];
                match rf.start(&flexbuffers::to_vec(&cmd).unwrap()) {
                    Ok(start) => {
                        index = Some(start.index);
                        break;
                    }
                    Err(e) => debug!("start cmd {:?} failed: {:?}", cmd, e),
                }
            }

            if let Some(index) = index {
                // somebody claimed to be the leader and to have
                // submitted our command; wait a while for agreement.
                let t1 = Instant::now();
                while t1.elapsed() < Duration::from_secs(2) {
                    let (nd, cmd1) = self.n_committed(index);
                    if nd > 0 && nd >= expected_servers {
                        // committed
                        if let Some(cmd2) = cmd1 {
                            if cmd2 == cmd {
                                // and it was the command we submitted.
                                return index;
                            }
                        }
                    }
                    time::sleep(Duration::from_millis(20)).await;
                }
                if !retry {
                    panic!("one({:?}) failed to reach agreement", cmd);
                }
            } else {
                time::sleep(Duration::from_millis(50)).await;
            }
        }
        panic!("one({:?}) failed to reach agreement", cmd);
    }

    /// detach server i from the net.
    pub fn disconnect(&mut self, i: usize) {
        debug!("disconnect({})", i);
        self.connected[i] = false;
        self.handle.net.disconnect(self.addrs[i]);
    }

    /// attach server i to the net.
    pub fn connect(&mut self, i: usize) {
        debug!("connect({})", i);
        self.connected[i] = true;
        self.handle.net.connect(self.addrs[i]);
    }

    /// Start or re-start a Raft.
    pub async fn start1(&mut self, i: usize) {
        self.rafts[i] = self.start1_ext(i, false).await;
    }

    /// Start or re-start a Raft with snapshot.
    pub async fn start1_snapshot(&mut self, i: usize) {
        self.rafts[i] = self.start1_ext(i, true).await;
    }

    async fn start1_ext(&mut self, i: usize, snapshot: bool) -> RaftHandle {
        self.crash1(i);

        self.handle.net.connect(self.addrs[i]);

        let addrs = self.addrs.clone();
        let (raft, mut apply_recver) = self
            .handle
            .local_handle(self.addrs[i])
            .spawn(async move { RaftHandle::new(addrs, i) })
            .await;
        let ret = raft.clone();

        // listen to messages from Raft indicating newly committed messages.
        let storage = self.storage.clone();
        madsim::task::spawn(async move {
            while let Some(cmd) = apply_recver.next().await {
                match cmd {
                    ApplyMsg::Command { data, index } => {
                        // debug!("apply {}", index);
                        let entry = flexbuffers::from_slice(&data)
                            .expect("committed command is not an entry");
                        storage.push_and_check(i, index, entry);
                        if snapshot && (index + 1) % SNAPSHOT_INTERVAL == 0 {
                            raft.snapshot(index, &data);
                        }
                    }
                    ApplyMsg::Snapshot { data, index, term } if snapshot => {
                        // debug!("install snapshot {}", index);
                        if raft.cond_install_snapshot(term, index, &data) {
                            todo!();
                        }
                    }
                    // ignore other types of ApplyMsg
                    _ => {}
                }
            }
        })
        .detach();

        ret
    }

    pub fn crash1(&self, i: usize) {
        self.handle.kill(self.addrs[i]);
    }

    /// End a test.
    ///
    /// The fact that we got here means there was no failure.
    /// Print the Passed message, and some performance numbers.
    pub fn end(self) {
        self.check_timeout();

        // real time
        let t = self.t0.elapsed();
        // number of RPC sends
        let nrpc = self.rpc_total() - self.rpc0;
        // number of Raft agreements reported
        let ncmds = self.storage.max_index();

        info!("  ... Passed --");
        info!("  {:?}  {} {} {}", t, self.n, nrpc, ncmds);
    }

    fn check_timeout(&self) {
        // enforce a two minute real-time limit on each test
        if self.t0.elapsed() > Duration::from_secs(120) {
            panic!("test took longer than 120 seconds");
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entry {
    pub x: u64,
}

#[derive(Clone)]
struct StorageHandle {
    /// copy of each server's committed entries
    logs: Arc<Mutex<Vec<Vec<Entry>>>>,
}

impl StorageHandle {
    fn new(n: usize) -> Self {
        StorageHandle {
            logs: Arc::new(Mutex::new(vec![vec![Entry { x: 0 }]; n])),
        }
    }

    fn push_and_check(&self, i: usize, index: u64, entry: Entry) {
        let mut logs = self.logs.lock().unwrap();
        for (j, log) in logs.iter().enumerate() {
            if let Some(old) = log.get(index as usize) {
                // some server has already committed a different value for this entry!
                assert_eq!(
                    *old, entry,
                    "commit index={:?} server={:?} {:?} != server={:?} {:?}",
                    index, i, entry, j, old
                );
            }
        }
        let log = &mut logs[i];
        if log.len() != index as usize {
            panic!("server {} apply out of order {}", i, index);
        }
        log.push(entry);
    }

    /// How many servers think a log entry is committed?
    fn n_committed(&self, index: u64) -> (usize, Option<Entry>) {
        let mut count = 0;
        let mut cmd = None;
        for log in self.logs.lock().unwrap().iter() {
            let cmd1 = log.get(index as usize).cloned();
            if cmd1.is_some() {
                if count > 0 && cmd != cmd1 {
                    panic!(
                        "committed values do not match: index {:?}, {:?}, {:?}",
                        index, cmd, cmd1
                    );
                }
                count += 1;
                cmd = cmd1;
            }
        }
        (count, cmd)
    }

    fn max_index(&self) -> usize {
        let logs = self.logs.lock().unwrap();
        logs.iter().map(|log| log.len() - 1).max().unwrap()
    }
}
