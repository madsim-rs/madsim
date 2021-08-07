use crate::tester::*;
use log::*;
use madsim::{
    rand::{self, Rng},
    time,
};
use std::time::Duration;

/// The tester generously allows solutions to complete elections in one second
/// (much more than the paper's range of timeouts).
const RAFT_ELECTION_TIMEOUT: Duration = Duration::from_millis(1000);

#[madsim::test]
async fn initial_election_2a() {
    let servers = 3;
    let mut t = RaftTester::new(servers).await;

    info!("Test (2A): initial election");

    // is a leader elected?
    t.check_one_leader().await;

    // sleep a bit to avoid racing with followers learning of the
    // election, then check that all peers agree on the term.
    time::sleep(Duration::from_millis(50)).await;
    let term1 = t.check_terms();

    // does the leader+term stay the same if there is no network failure?
    time::sleep(2 * RAFT_ELECTION_TIMEOUT).await;
    let term2 = t.check_terms();
    if term1 != term2 {
        warn!("warning: term changed even though there were no failures")
    }

    // there should still be a leader.
    t.check_one_leader().await;

    t.end();
}

#[madsim::test]
async fn reelection_2a() {
    let servers = 3;
    let mut t = RaftTester::new(servers).await;
    info!("Test (2A): election after network failure");

    let leader1 = t.check_one_leader().await;
    // if the leader disconnects, a new one should be elected.
    t.disconnect(leader1);
    t.check_one_leader().await;

    // if the old leader rejoins, that shouldn't disturb the new leader.
    t.connect(leader1);
    let leader2 = t.check_one_leader().await;

    // if there's no quorum, no leader should be elected.
    t.disconnect(leader2);
    t.disconnect((leader2 + 1) % servers);
    time::sleep(2 * RAFT_ELECTION_TIMEOUT).await;
    t.check_no_leader();

    // if a quorum arises, it should elect a leader.
    t.connect((leader2 + 1) % servers);
    t.check_one_leader().await;

    // re-join of last node shouldn't prevent leader from existing.
    t.connect(leader2);
    t.check_one_leader().await;

    t.end();
}

#[madsim::test]
async fn many_election_2a() {
    let servers = 7;
    let iters = 10;
    let mut t = RaftTester::new(servers).await;

    info!("Test (2A): multiple elections");

    t.check_one_leader().await;

    let mut random = rand::rng();
    for _ in 0..iters {
        // disconnect three nodes
        let i1 = random.gen_range(0..servers);
        let i2 = random.gen_range(0..servers);
        let i3 = random.gen_range(0..servers);
        t.disconnect(i1);
        t.disconnect(i2);
        t.disconnect(i3);

        // either the current leader should still be alive,
        // or the remaining four should elect a new one.
        t.check_one_leader().await;

        t.connect(i1);
        t.connect(i2);
        t.connect(i3);
    }

    t.check_one_leader().await;

    t.end();
}

#[madsim::test]
async fn basic_agree_2b() {
    let servers = 5;
    let mut t = RaftTester::new(servers).await;
    info!("Test (2B): basic agreement");

    let iters = 3;
    for index in 1..=iters {
        let (nd, _) = t.n_committed(index);
        assert_eq!(nd, 0, "some have committed before start()");

        let xindex = t.one(Entry { x: index * 100 }, servers, false).await;
        assert_eq!(xindex, index, "got index {} but expected {}", xindex, index);
    }

    t.end();
}

#[madsim::test]
async fn fail_agree_2b() {
    let servers = 3;
    let mut t = RaftTester::new(servers).await;

    info!("Test (2B): agreement despite follower disconnection");

    t.one(Entry { x: 101 }, servers, false).await;

    // follower network disconnection
    let leader = t.check_one_leader().await;
    t.disconnect((leader + 1) % servers);

    // agree despite one disconnected server?
    t.one(Entry { x: 102 }, servers - 1, false).await;
    t.one(Entry { x: 103 }, servers - 1, false).await;
    time::sleep(RAFT_ELECTION_TIMEOUT).await;
    t.one(Entry { x: 104 }, servers - 1, false).await;
    t.one(Entry { x: 105 }, servers - 1, false).await;

    // re-connect
    t.connect((leader + 1) % servers);

    // agree with full set of servers?
    t.one(Entry { x: 106 }, servers, true).await;
    time::sleep(RAFT_ELECTION_TIMEOUT).await;
    t.one(Entry { x: 107 }, servers, true).await;

    t.end();
}

#[madsim::test]
async fn fail_no_agree_2b() {
    let servers = 5;
    let mut t = RaftTester::new(servers).await;

    info!("Test (2B): no agreement if too many followers disconnect");

    t.one(Entry { x: 10 }, servers, false).await;

    // 3 of 5 followers disconnect
    let leader = t.check_one_leader().await;
    t.disconnect((leader + 1) % servers);
    t.disconnect((leader + 2) % servers);
    t.disconnect((leader + 3) % servers);
    let index = t
        .start(leader, Entry { x: 20 })
        .expect("leader rejected start")
        .index;
    if index != 2 {
        panic!("expected index 2, got {}", index);
    }

    time::sleep(2 * RAFT_ELECTION_TIMEOUT).await;

    let (n, _) = t.n_committed(index);
    assert_eq!(n, 0, "{} committed but no majority", n);

    // repair
    t.connect((leader + 1) % servers);
    t.connect((leader + 2) % servers);
    t.connect((leader + 3) % servers);

    // the disconnected majority may have chosen a leader from
    // among their own ranks, forgetting index 2.
    let leader2 = t.check_one_leader().await;
    let index2 = t
        .start(leader2, Entry { x: 30 })
        .expect("leader2 rejected start")
        .index;
    assert!((2..=3).contains(&index2), "unexpected index {}", index2);

    t.one(Entry { x: 1000 }, servers, true).await;

    t.end();
}

#[madsim::test]
async fn concurrent_starts_2b() {
    let servers = 3;
    let mut t = RaftTester::new(servers).await;

    info!("Test (2B): concurrent start()s");
    let mut success = false;
    'outer: for tried in 0..5 {
        if tried > 0 {
            // give solution some time to settle
            time::sleep(Duration::from_secs(3)).await;
        }

        let leader = t.check_one_leader().await;
        let term = match t.start(leader, Entry { x: 1 }) {
            Err(err) => {
                warn!("start leader {} meet error {:?}", leader, err);
                continue;
            }
            Ok(start) => start.term,
        };

        let mut idxes = vec![];
        for ii in 0..5 {
            match t.start(leader, Entry { x: 100 + ii }) {
                Err(err) => {
                    warn!("start leader {} meet error {:?}", leader, err);
                }
                Ok(start) => {
                    if start.term == term {
                        idxes.push(start.index);
                    }
                }
            };
        }

        if (0..servers).any(|j| t.term(j) != term) {
            // term changed -- can't expect low RPC counts
            continue 'outer;
        }

        let mut cmds = vec![];
        for index in idxes {
            if let Some(cmd) = t.wait(index, servers, Some(term)).await {
                cmds.push(cmd.x);
            } else {
                // peers have moved on to later terms
                // so we can't expect all Start()s to
                // have succeeded
                continue;
            }
        }
        for ii in 0..5 {
            let x: u64 = 100 + ii;
            let ok = cmds.iter().find(|&&cmd| cmd == x).is_some();
            assert!(ok, "cmd {} missing in {:?}", x, cmds);
        }
        success = true;
        break;
    }

    assert!(success, "term changed too often");

    t.end();
}

#[madsim::test]
async fn rejoin_2b() {
    let servers = 3;
    let mut t = RaftTester::new(servers).await;

    info!("Test (2B): rejoin of partitioned leader");

    t.one(Entry { x: 101 }, servers, true).await;

    // leader network failure
    let leader1 = t.check_one_leader().await;
    t.disconnect(leader1);

    // make old leader try to agree on some entries
    let _ = t.start(leader1, Entry { x: 102 });
    let _ = t.start(leader1, Entry { x: 103 });
    let _ = t.start(leader1, Entry { x: 104 });

    // new leader commits, also for index=2
    t.one(Entry { x: 103 }, 2, true).await;

    // new leader network failure
    let leader2 = t.check_one_leader().await;
    t.disconnect(leader2);

    // old leader connected again
    t.connect(leader1);

    t.one(Entry { x: 104 }, 2, true).await;

    // all together now
    t.connect(leader2);

    t.one(Entry { x: 105 }, servers, true).await;

    t.end();
}

#[madsim::test]
async fn backup_2b() {
    let servers = 5;
    let mut t = RaftTester::new(servers).await;

    info!("Test (2B): leader backs up quickly over incorrect follower logs");

    let mut random = rand::rng();
    t.one(random.gen_entry(), servers, true).await;

    // put leader and one follower in a partition
    let leader1 = t.check_one_leader().await;
    t.disconnect((leader1 + 2) % servers);
    t.disconnect((leader1 + 3) % servers);
    t.disconnect((leader1 + 4) % servers);

    // submit lots of commands that won't commit
    for _i in 0..50 {
        let _ = t.start(leader1, random.gen_entry());
    }

    time::sleep(RAFT_ELECTION_TIMEOUT / 2).await;

    t.disconnect((leader1 + 0) % servers);
    t.disconnect((leader1 + 1) % servers);

    // allow other partition to recover
    t.connect((leader1 + 2) % servers);
    t.connect((leader1 + 3) % servers);
    t.connect((leader1 + 4) % servers);

    // lots of successful commands to new group.
    for _i in 0..50 {
        t.one(random.gen_entry(), 3, true).await;
    }

    // now another partitioned leader and one follower
    let leader2 = t.check_one_leader().await;
    let mut other = (leader1 + 2) % servers;
    if leader2 == other {
        other = (leader2 + 1) % servers;
    }
    t.disconnect(other);

    // lots more commands that won't commit
    for _i in 0..50 {
        let _ = t.start(leader2, random.gen_entry());
    }

    time::sleep(RAFT_ELECTION_TIMEOUT / 2).await;

    // bring original leader back to life,
    for i in 0..servers {
        t.disconnect(i);
    }
    t.connect((leader1 + 0) % servers);
    t.connect((leader1 + 1) % servers);
    t.connect(other);

    // lots of successful commands to new group.
    for _i in 0..50 {
        t.one(random.gen_entry(), 3, true).await;
    }

    // now everyone
    for i in 0..servers {
        t.connect(i);
    }
    t.one(random.gen_entry(), servers, true).await;

    t.end();
}

#[madsim::test]
async fn count_2b() {
    let servers = 3;
    let mut t = RaftTester::new(servers).await;
    info!("Test (2B): RPC counts aren't too high");

    t.check_one_leader().await;
    let mut total1 = t.rpc_total();

    assert!(
        (1..=30).contains(&total1),
        "too many or few RPCs ({}) to elect initial leader",
        total1
    );

    let mut total2 = 0;
    let mut success = false;
    let mut random = rand::rng();
    'outer: for tried in 0..5 {
        if tried > 0 {
            // give solution some time to settle
            time::sleep(Duration::from_secs(3)).await;
        }

        let leader = t.check_one_leader().await;
        total1 = t.rpc_total();

        let iters = 10;
        let (starti, term) = match t.start(leader, Entry { x: 1 }) {
            Ok(s) => (s.index, s.term),
            Err(err) => {
                warn!("start leader {} meet error {:?}", leader, err);
                continue;
            }
        };

        let mut cmds = vec![];
        for i in 1..iters + 2 {
            let x = random.gen::<u64>();
            cmds.push(x);
            match t.start(leader, Entry { x }) {
                Ok(s) => {
                    if s.term != term {
                        // Term changed while starting
                        continue 'outer;
                    }
                    assert_eq!(starti + i, s.index, "start failed");
                }
                Err(err) => {
                    warn!("start leader {} meet error {:?}", leader, err);
                    continue 'outer;
                }
            }
        }

        for i in 1..=iters {
            if let Some(ix) = t.wait(starti + i, servers, Some(term)).await {
                assert_eq!(
                    ix.x,
                    cmds[(i - 1) as usize],
                    "wrong value {:?} committed for index {}; expected {:?}",
                    ix,
                    starti + i,
                    cmds
                );
            }
        }

        if (0..servers).any(|i| t.term(i) != term) {
            // term changed -- can't expect low RPC counts
            continue 'outer;
        }
        total2 = t.rpc_total();
        if total2 - total1 > (iters as u64 + 1 + 3) * 3 {
            panic!("too many RPCs ({}) for {} entries", total2 - total1, iters);
        }

        success = true;
        break;
    }
    assert!(success, "term changed too often");

    time::sleep(RAFT_ELECTION_TIMEOUT).await;
    let total3 = t.rpc_total();
    assert!(
        total3 - total2 <= 3 * 20,
        "too many RPCs ({}) for 1 second of idleness",
        total3 - total2
    );

    t.end();
}

trait GenEntry {
    fn gen_entry(&mut self) -> Entry;
}

impl<R: Rng> GenEntry for R {
    fn gen_entry(&mut self) -> Entry {
        Entry { x: self.gen() }
    }
}
