use crate::raft::*;
use madsim::{
    rand::{self, Rng},
    time, Handle,
};
use std::{
    collections::HashMap,
    net::SocketAddr,
    time::{Duration, Instant},
};
use log::*;

pub struct RaftTester {
    handle: Handle,
    n: usize,
    addrs: Vec<SocketAddr>,
    rafts: Vec<RaftHandle>,
    connected: Vec<bool>,
    start_time: Instant,
}

impl RaftTester {
    pub async fn new(n: usize) -> Self {
        let handle = Handle::current();
        let addrs = (0..n)
            .map(|i| SocketAddr::from(([0, 0, 0, i as _], 0)))
            .collect::<Vec<_>>();
        let connected = (0..n).map(|_| true).collect();
        let mut rafts = vec![];
        for i in 0..n {
            let addrs = addrs.clone();
            let raft = handle
                .local_handle(addrs[i])
                .spawn(async move { RaftHandle::new(addrs, i) })
                .await;
            rafts.push(raft);
        }
        RaftTester {
            handle,
            n,
            addrs,
            rafts,
            connected,
            start_time: time::now(),
        }
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

    pub fn crash(&self, i: usize) {
        self.handle.kill(self.addrs[i]);
    }

    /// End a test.
    ///
    /// The fact that we got here means there was no failure.
    /// Print the Passed message, and some performance numbers.
    pub fn end(&self) {
        self.check_timeout();

        // real time
        let t = time::now() - self.start_time;
        // number of RPC sends
        let nrpc = self.handle.net.msg_count();

        // number of Raft agreements reported
        // let s = self.storage.lock().unwrap();
        // let ncmds = s.max_index - s.max_index0;
        let ncmds = 0;

        info!("  ... Passed --");
        info!("  {:?}  {} {} {}", t, self.n, nrpc, ncmds);
    }

    pub fn check_timeout(&self) {
        // enforce a two minute real-time limit on each test
        if time::now() - self.start_time > Duration::from_secs(120) {
            panic!("test took longer than 120 seconds");
        }
    }
}
