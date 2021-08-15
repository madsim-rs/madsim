use madsim::{time::*, Handle, LocalHandle};
use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::atomic::{AtomicUsize, Ordering},
    sync::{Arc, Mutex},
};

use super::{client, msg::Config, server};

pub struct Tester {
    handle: Handle,
    n: usize,
    addrs: Vec<SocketAddr>,
    servers: Mutex<Vec<Option<Arc<server::ShardCtrler>>>>,
    next_client_id: AtomicUsize,

    // begin()/end() statistics
    t0: Instant,
    // rpc_total() at start of test
    rpcs0: u64,
    // number of agreements
    ops: Arc<AtomicUsize>,
}

impl Tester {
    pub async fn new(n: usize, unreliable: bool) -> Tester {
        let handle = Handle::current();
        if unreliable {
            handle.net.update_config(|cfg| {
                cfg.packet_loss_rate = 0.1;
                cfg.send_latency = Duration::from_millis(1)..Duration::from_millis(27);
            });
        }
        let mut servers = vec![];
        servers.resize_with(n, || None);
        let mut tester = Tester {
            handle,
            n,
            addrs: (0..n)
                .map(|i| SocketAddr::from(([0, 0, 1, i as _], 0)))
                .collect::<Vec<_>>(),
            servers: Mutex::new(servers),
            next_client_id: AtomicUsize::new(0),
            t0: Instant::now(),
            rpcs0: 0,
            ops: Arc::new(AtomicUsize::new(0)),
        };
        tester.rpcs0 = tester.rpc_total();
        // create a full set of KV servers.
        for i in 0..n {
            tester.start_server(i).await;
        }
        tester
    }

    fn rpc_total(&self) -> u64 {
        self.handle.net.stat().msg_count / 2
    }

    fn check_timeout(&self) {
        // enforce a two minute real-time limit on each test
        if self.t0.elapsed() > Duration::from_secs(120) {
            panic!("test took longer than 120 seconds");
        }
    }

    /// Maximum log size across all servers
    pub fn log_size(&self) -> usize {
        self.addrs
            .iter()
            .map(|&addr| self.handle.fs.get_file_size(addr, "state").unwrap())
            .max()
            .unwrap() as usize
    }

    /// Maximum snapshot size across all servers
    pub fn snapshot_size(&self) -> usize {
        self.addrs
            .iter()
            .map(|&addr| self.handle.fs.get_file_size(addr, "snapshot").unwrap())
            .max()
            .unwrap() as usize
    }

    /// Attach server i to servers listed in to
    fn connect(&self, i: usize, to: &[usize]) {
        debug!("connect peer {} to {:?}", i, to);
        for &j in to {
            self.handle.net.connect2(self.addrs[i], self.addrs[j]);
        }
    }

    /// Detach server i from the servers listed in from
    fn disconnect(&self, i: usize, from: &[usize]) {
        debug!("disconnect peer {} from {:?}", i, from);
        for &j in from {
            self.handle.net.disconnect2(self.addrs[i], self.addrs[j]);
        }
    }

    pub fn all(&self) -> Vec<usize> {
        (0..self.n).collect()
    }

    pub fn connect_all(&self) {
        for i in 0..self.n {
            self.connect(i, &self.all());
        }
    }

    /// Sets up 2 partitions with connectivity between servers in each  partition.
    pub fn partition(&self, p1: &[usize], p2: &[usize]) {
        debug!("partition servers into: {:?} {:?}", p1, p2);
        for &i in p1 {
            self.disconnect(i, p2);
            self.connect(i, p1);
        }
        for &i in p2 {
            self.disconnect(i, p1);
            self.connect(i, p2);
        }
    }

    // Create a clerk with clerk specific server names.
    // Give it connections to all of the servers, but for
    // now enable only connections to servers in to[].
    pub fn make_client(&self, to: &[usize]) -> Clerk {
        let id = ClerkId(self.next_client_id.fetch_add(1, Ordering::SeqCst));
        self.connect_client(id, to);
        Clerk {
            id,
            handle: self.handle.local_handle(id.to_addr()),
            ck: Arc::new(client::Clerk::new(self.addrs.clone())),
            ops: self.ops.clone(),
        }
    }

    pub fn connect_client(&self, id: ClerkId, to: &[usize]) {
        debug!("connect {:?} to {:?}", id, to);
        let addr = id.to_addr();
        self.handle.net.connect(addr);
        for i in 0..self.n {
            self.handle.net.disconnect2(addr, self.addrs[i]);
        }
        for &i in to {
            self.handle.net.connect2(addr, self.addrs[i]);
        }
    }

    /// Shutdown a server.
    pub fn shutdown_server(&self, i: usize) {
        debug!("shutdown_server({})", i);
        self.handle.kill(self.addrs[i]);
        self.servers.lock().unwrap()[i] = None;
    }

    /// Start a server.
    /// If restart servers, first call shutdown_server
    pub async fn start_server(&self, i: usize) {
        debug!("start_server({})", i);
        let addrs = self.addrs.clone();
        let handle = self.handle.local_handle(self.addrs[i]);
        let kv = handle.spawn(server::ShardCtrler::new(addrs, i, None)).await;
        self.servers.lock().unwrap()[i] = Some(kv);
    }

    pub fn leader(&self) -> Option<usize> {
        let servers = self.servers.lock().unwrap();
        for (i, kv) in servers.iter().enumerate() {
            if let Some(kv) = kv {
                if kv.is_leader() {
                    return Some(i);
                }
            }
        }
        None
    }

    /// Partition servers into 2 groups and put current leader in minority
    pub fn make_partition(&self) -> (Vec<usize>, Vec<usize>) {
        let leader = self.leader().unwrap_or(0);
        let mut p1 = (0..self.n).collect::<Vec<usize>>();
        p1.swap_remove(leader);
        let mut p2 = p1.split_off(self.n / 2 + 1);
        p2.push(leader);
        (p1, p2)
    }

    /// End a Test -- the fact that we got here means there
    /// was no failure.
    /// print the Passed message,
    /// and some performance numbers.
    pub fn end(&self) {
        self.check_timeout();

        // real time
        let t = self.t0.elapsed();
        // number of Raft peers
        let npeers = self.n;
        // number of RPC sends
        let nrpc = self.rpc_total() - self.rpcs0;
        // number of clerk get/put/append calls
        let nops = self.ops.load(Ordering::Relaxed);

        info!("  ... Passed --");
        info!("  {:?}  {} {} {}", t, npeers, nrpc, nops);
    }
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct ClerkId(usize);

impl ClerkId {
    fn to_addr(&self) -> SocketAddr {
        SocketAddr::from(([0, 0, 2, self.0 as u8], 1))
    }
}

pub struct Clerk {
    handle: LocalHandle,
    id: ClerkId,
    ck: Arc<client::Clerk>,
    ops: Arc<AtomicUsize>,
}

impl Clerk {
    pub const fn id(&self) -> ClerkId {
        self.id
    }

    pub async fn query(&self) -> Config {
        self.op();
        let ck = self.ck.clone();
        self.handle.spawn(async move { ck.query(None).await }).await
    }

    pub async fn query_at(&self, num: u64) -> Config {
        debug!("query_at: {}", num);
        self.op();
        let ck = self.ck.clone();
        self.handle
            .spawn(async move { ck.query(Some(num)).await })
            .await
    }

    pub async fn join(&self, groups: HashMap<u64, Vec<String>>) {
        debug!("join: {:?}", groups);
        self.op();
        let ck = self.ck.clone();
        self.handle
            .spawn(async move { ck.join(groups).await })
            .await
    }

    pub async fn leave(&self, gids: &[u64]) {
        debug!("leave: {:?}", gids);
        self.op();
        let ck = self.ck.clone();
        let gids = Vec::from(gids);
        self.handle.spawn(async move { ck.leave(gids).await }).await
    }

    pub async fn move_(&self, shard: usize, gid: u64) {
        debug!("move: shard {} -> gid {}", shard, gid);
        self.op();
        let ck = self.ck.clone();
        self.handle
            .spawn(async move { ck.move_(shard, gid).await })
            .await
    }

    pub async fn check(&self, groups: &[u64]) {
        debug!("check: {:?}", groups);
        let c = self.query().await;
        assert_eq!(c.groups.len(), groups.len());
        // are the groups as expected?
        for gid in groups {
            assert!(c.groups.contains_key(gid), "missing group {}", gid);
        }
        // any un-allocated shards?
        if groups.is_empty() {
            for (shard, gid) in c.shards.iter() {
                assert!(
                    c.groups.contains_key(gid),
                    "shard {} -> invalid group {}",
                    shard,
                    gid
                );
            }
        }
        // more or less balanced sharding?
        let mut counts = HashMap::<u64, usize>::new();
        for (_, &gid) in c.shards.iter() {
            *counts.entry(gid).or_default() += 1;
        }
        if !c.groups.is_empty() {
            let counts = c.groups.keys().map(|gid| *counts.get(gid).unwrap_or(&0));
            let min = counts.clone().min().unwrap();
            let max = counts.clone().max().unwrap();
            assert!(
                max <= min + 1,
                "imbalanced sharding, max {} too much larger than min {}",
                max,
                min
            );
        }
    }

    fn op(&self) {
        self.ops.fetch_add(1, Ordering::Relaxed);
    }
}
