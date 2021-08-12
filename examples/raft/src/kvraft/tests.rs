use super::tester::Tester;
use futures::{future, select, FutureExt};
use madsim::{
    rand::{self, seq::SliceRandom, Rng},
    task,
    time::{self, Duration},
};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

/// The tester generously allows solutions to complete elections in one second
/// (much more than the paper's range of timeouts).
const RAFT_ELECTION_TIMEOUT: Duration = Duration::from_millis(1000);

// const LINEARIZABILITY_CHECK_TIMEOUT: Duration = Duration::from_millis(1000);

// check that for a specific client all known appends are present in a value,
// and in order
fn check_clnt_appends(clnt: usize, v: &str, count: usize) {
    let mut lastoff = None;
    for j in 0..count {
        let wanted = format!("x {} {} y", clnt, j);
        let off = v.find(&wanted).unwrap_or_else(|| {
            panic!(
                "{:?} missing element {:?} in Append result {:?}",
                clnt, wanted, v
            )
        });
        let off1 = v.rfind(&wanted).unwrap();
        assert_eq!(off1, off, "duplicate element {:?} in Append result", wanted);

        if let Some(lastoff) = lastoff {
            assert!(
                off > lastoff,
                "wrong order for element {:?} in Append result",
                wanted
            );
        }
        lastoff = Some(off);
    }
}

// check that all known appends are present in a value,
// and are in order for each concurrent client.
fn check_concurrent_appends(v: &str, counts: &[usize]) {
    for (i, &count) in counts.iter().enumerate() {
        check_clnt_appends(i, v, count);
    }
}

/// Basic test is as follows:
///
/// One or more clients submitting Append/Get operations to set of servers for some period of time.
/// After the period is over, test checks that all appended values are present and in order for a
/// particular key.  
///
/// - If unreliable is set, RPCs may fail.
/// - If crash is set, the servers crash after the period is over and restart.
/// - If partitions is set, the test repartitions the network concurrently with
///   the clients and servers.
/// - If maxraftstate is a positive number, the size of the state for Raft
///   (i.e., log size) shouldn't exceed `2 * maxraftstate`.
async fn generic_test(
    part: &str,
    nclients: usize,
    unreliable: bool,
    crash: bool,
    partitions: bool,
    maxraftstate: Option<usize>,
) {
    let mut title = "Test: ".to_owned();
    if unreliable {
        // the network drops RPC requests and replies.
        title += "unreliable net, ";
    }
    if crash {
        // peers re-start, and thus persistence must work.
        title += "restarts, ";
    }
    if partitions {
        // the network may partition
        title += "partitions, ";
    }
    if maxraftstate.is_some() {
        title += "snapshots, ";
    }
    if nclients > 1 {
        title += "many clients";
    } else {
        title += "one client";
    }
    info!("{} ({})", title, part);

    const NSERVERS: usize = 5;
    let t = Arc::new(Tester::new(NSERVERS, unreliable, maxraftstate).await);

    let ck = t.make_client(&t.all());

    for i in 0..3 {
        debug!("Iteration {}", i);
        let done = Arc::new(AtomicBool::new(false));

        let mut cas = vec![];
        for cli in 0..nclients {
            let ck = t.make_client(&t.all());
            let done = done.clone();
            cas.push(task::spawn_local(async move {
                // TODO: change the closure to a future.
                let mut j = 0;
                let mut rng = rand::rng();
                let mut last = String::new();
                let key = format!("{}", cli);
                ck.put(&key, &last).await;
                while !done.load(Ordering::Relaxed) {
                    if rng.gen_bool(0.5) {
                        let nv = format!("x {} {} y", cli, j);
                        debug!("{}: client new append {:?}", cli, nv);
                        // predict effect of append(k, val) if old value is prev.
                        last += &nv;
                        ck.append(&key, &nv).await;
                        j += 1;
                    } else {
                        debug!("{}: client new get {:?}", cli, key);
                        let v = ck.get(&key).await;
                        assert_eq!(v, last, "get wrong value, key {:?}", key);
                    }
                }
                j
            }));
        }

        let partitioner = if partitions {
            // Allow the clients to perform some operations without interruption
            time::sleep(Duration::from_secs(1)).await;

            let t = t.clone();
            let done = done.clone();
            // repartition the servers periodically
            Some(task::spawn_local(async move {
                let mut all = t.all();
                let n = all.len();
                let mut rng = rand::rng();
                while !done.load(Ordering::Relaxed) {
                    rng.with(|rng| all.shuffle(rng));
                    let (left, right) = all.split_at(rng.gen_range(0..n));
                    t.partition(left, right);
                    time::sleep(
                        RAFT_ELECTION_TIMEOUT + Duration::from_millis(rng.gen_range(0..200)),
                    )
                    .await;
                }
            }))
        } else {
            None
        };
        time::sleep(Duration::from_secs(5)).await;

        // tell clients and partitioner to quit
        done.store(true, Ordering::Relaxed);

        if let Some(partitioner) = partitioner {
            debug!("wait for partitioner");
            partitioner.await;
            // reconnect network and submit a request. A client may
            // have submitted a request in a minority.  That request
            // won't return until that server discovers a new term
            // has started.
            t.connect_all();
            // wait for a while so that we have a new term
            time::sleep(RAFT_ELECTION_TIMEOUT).await;
        }

        if crash {
            debug!("shutdown servers");
            for i in 0..NSERVERS {
                t.shutdown_server(i);
            }
            // Wait for a while for servers to shutdown, since
            // shutdown isn't a real crash and isn't instantaneous
            time::sleep(RAFT_ELECTION_TIMEOUT).await;
            debug!("restart servers");
            // crash and re-start all
            for i in 0..NSERVERS {
                t.start_server(i).await;
            }
            t.connect_all();
        }

        debug!("wait for clients");
        for (i, task) in cas.into_iter().enumerate() {
            debug!("read from clients {}", i);
            let j = task.await;
            if j < 10 {
                warn!(
                    "client {} managed to perform only {} put operations in 1 sec?",
                    i, j
                );
            }
            let key = format!("{}", i);
            debug!("Check {:?} for client {}", j, i);
            let v = ck.get(&key).await;
            check_clnt_appends(i, &v, j);
        }

        if let Some(maxraftstate) = maxraftstate {
            // Check maximum after the servers have processed all client
            // requests and had time to checkpoint.
            assert!(
                t.log_size() <= 2 * maxraftstate,
                "logs were not trimmed ({} > 2*{})",
                t.log_size(),
                maxraftstate
            )
        }
    }

    t.end();
}

#[madsim::test]
async fn test_basic_3a() {
    // Test: one client (3A) ...
    generic_test("3A", 1, false, false, false, None).await;
}

#[madsim::test]
async fn test_concurrent_3a() {
    // Test: many clients (3A) ...
    generic_test("3A", 5, false, false, false, None).await;
}

#[madsim::test]
async fn test_unreliable_3a() {
    // Test: unreliable net, many clients (3A) ...
    generic_test("3A", 5, true, false, false, None).await;
}

#[madsim::test]
async fn test_unreliable_one_key_3a() {
    let nservers = 3;
    let t = Tester::new(nservers, true, None).await;
    info!("Test: concurrent append to same key, unreliable (3A)");

    let all = t.all();
    let ck = t.make_client(&all);

    ck.put("k", "").await;

    let nclient = 5;
    let upto = 10;

    let mut cas = vec![];
    for i in 0..nclient {
        let ck = t.make_client(&t.all());
        cas.push(task::spawn_local(async move {
            for n in 0..upto {
                ck.append("k", &format!("x {} {} y", i, n)).await;
            }
        }));
    }
    future::join_all(cas).await;

    let counts = vec![upto; nclient];

    let vx = ck.get("k").await;
    check_concurrent_appends(&vx, &counts);

    t.end();
}

// Submit a request in the minority partition and check that the requests
// doesn't go through until the partition heals. The leader in the original
// network ends up in the minority partition.
#[madsim::test]
async fn test_one_partition_3a() {
    let nservers = 5;
    let t = Tester::new(nservers, false, None).await;

    let all = t.all();
    let ck = t.make_client(&all);

    ck.put("1", "13").await;

    info!("Test: progress in majority (3A)");

    let (p1, p2) = t.make_partition();
    t.partition(&p1, &p2);

    // connect ckp1 to p1
    let ckp1 = t.make_client(&p1);
    // connect ckp2a to p2
    let ckp2a = t.make_client(&p2);
    let ckp2a_name = ckp2a.id();
    // connect ckp2b to p2
    let ckp2b = t.make_client(&p2);
    let ckp2b_name = ckp2b.id();

    ckp1.put("1", "14").await;
    ckp1.check("1", "14").await;

    info!("Test: no progress in minority (3A)");

    let mut put = task::spawn_local(async move {
        ckp2a.put("1", "15").await;
    })
    .fuse();
    let mut get = task::spawn_local(async move {
        // different clerk in p2
        ckp2b.get("1").await;
    })
    .fuse();

    select! {
        _ = put => panic!("put in minority completed"),
        _ = get => panic!("get in minority completed"),
        _ = time::sleep(Duration::from_secs(1)).fuse() => {}
    }

    ckp1.check("1", "14").await;
    ckp1.put("1", "16").await;
    ckp1.check("1", "16").await;

    info!("Test: completion after heal (3A)");

    t.connect_all();
    t.connect_client(ckp2a_name, &all);
    t.connect_client(ckp2b_name, &all);

    time::sleep(RAFT_ELECTION_TIMEOUT).await;

    select! {
        _ = time::sleep(Duration::from_secs(3)).fuse() => panic!("put/get did not complete"),
        _ = put => info!("put completes"),
        _ = get => info!("get completes"),
    }

    ck.check("1", "15").await;

    t.end();
}

#[madsim::test]
async fn test_many_partitions_one_client_3a() {
    // Test: partitions, one client (3A) ...
    generic_test("3A", 1, false, false, true, None).await;
}

#[madsim::test]
async fn test_many_partitions_many_clients_3a() {
    // Test: partitions, many clients (3A) ...
    generic_test("3A", 5, false, false, true, None).await;
}

#[madsim::test]
async fn test_persist_one_client_3a() {
    // Test: restarts, one client (3A) ...
    generic_test("3A", 1, false, true, false, None).await;
}

#[madsim::test]
async fn test_persist_concurrent_3a() {
    // Test: restarts, many clients (3A) ...
    generic_test("3A", 5, false, true, false, None).await;
}

#[madsim::test]
async fn test_persist_concurrent_unreliable_3a() {
    // Test: unreliable net, restarts, many clients (3A) ...
    generic_test("3A", 5, true, true, false, None).await;
}

#[madsim::test]
async fn test_persist_partition_3a() {
    // Test: restarts, partitions, many clients (3A) ...
    generic_test("3A", 5, false, true, true, None).await;
}

#[madsim::test]
async fn test_persist_partition_unreliable_3a() {
    // Test: unreliable net, restarts, partitions, many clients (3A) ...
    generic_test("3A", 5, true, true, true, None).await;
}

// #[madsim::test]
// async fn test_persist_partition_unreliable_linearizable_3a() {
//     // Test: unreliable net, restarts, partitions, linearizability checks (3A) ...
//     generic_test_linearizability("3A", 15, 7, true, true, true, None)
// }

// if one server falls behind, then rejoins, does it
// recover by using the InstallSnapshot RPC?
// also checks that majority discards committed log entries
// even if minority doesn't respond.
#[madsim::test]
async fn test_snapshot_rpc_3b() {
    let nservers = 3;
    let maxraftstate = 1000;
    let t = Tester::new(nservers, false, Some(maxraftstate)).await;

    let all = t.all();
    let ck = t.make_client(&all);

    info!("Test: InstallSnapshot RPC (3B)");

    ck.put("a", "A").await;
    ck.check("a", "A").await;

    // a bunch of puts into the majority partition.
    t.partition(&[0, 1], &[2]);
    {
        let ck1 = t.make_client(&[0, 1]);
        for i in 0..50 {
            ck1.put(&format!("{}", i), &format!("{}", i)).await;
        }
        time::sleep(RAFT_ELECTION_TIMEOUT).await;
        ck1.put("b", "B").await;
    }

    // check that the majority partition has thrown away
    // most of its log entries.
    assert!(
        t.log_size() <= 2 * maxraftstate,
        "logs were not trimmed ({} > 2*{})",
        t.log_size(),
        maxraftstate,
    );

    // now make group that requires participation of
    // lagging server, so that it has to catch up.
    t.partition(&[0, 2], &[1]);
    {
        let ck1 = t.make_client(&[0, 2]);
        ck1.put("c", "C").await;
        ck1.put("d", "D").await;
        ck1.check("a", "A").await;
        ck1.check("b", "B").await;
        ck1.check("1", "1").await;
        ck1.check("49", "49").await;
    }

    // now everybody
    t.partition(&[0, 1, 2], &[]);

    ck.put("e", "E").await;
    ck.check("c", "C").await;
    ck.check("e", "E").await;
    ck.check("1", "1").await;

    t.end();
}

// are the snapshots not too huge? 500 bytes is a generous bound for the
// operations we're doing here.
#[madsim::test]
async fn test_snapshot_size_3b() {
    let nservers = 3;
    let maxraftstate = 1000;
    let maxsnapshotstate = 500;
    let t = Tester::new(nservers, false, Some(maxraftstate)).await;

    let all = t.all();
    let ck = t.make_client(&all);

    info!("Test: snapshot size is reasonable (3B)");

    for _ in 0..200 {
        ck.put("x", "0").await;
        ck.check("x", "0").await;
        ck.put("x", "1").await;
        ck.check("x", "1").await;
    }

    // check that servers have thrown away most of their log entries
    assert!(
        t.log_size() <= 2 * maxraftstate,
        "logs were not trimmed ({} > 2*{})",
        t.log_size(),
        maxraftstate,
    );

    // check that the snapshots are not unreasonably large
    assert!(
        t.snapshot_size() <= maxsnapshotstate,
        "snapshot too large ({} > {})",
        t.snapshot_size(),
        maxsnapshotstate,
    );

    t.end();
}

#[madsim::test]
async fn test_snapshot_recover_3b() {
    // Test: restarts, snapshots, one client (3B) ...
    generic_test("3B", 1, false, true, false, Some(1000)).await;
}

#[madsim::test]
async fn test_snapshot_recover_many_clients_3b() {
    // Test: restarts, snapshots, many clients (3B) ...
    generic_test("3B", 20, false, true, false, Some(1000)).await;
}

#[madsim::test]
async fn test_snapshot_unreliable_3b() {
    // Test: unreliable net, snapshots, many clients (3B) ...
    generic_test("3B", 5, true, false, false, Some(1000)).await;
}

#[madsim::test]
async fn test_snapshot_unreliable_recover_3b() {
    // Test: unreliable net, restarts, snapshots, many clients (3B) ...
    generic_test("3B", 5, true, true, false, Some(1000)).await;
}

#[madsim::test]
async fn test_snapshot_unreliable_recover_concurrent_partition_3b() {
    // Test: unreliable net, restarts, partitions, snapshots, many clients (3B) ...
    generic_test("3B", 5, true, true, true, Some(1000)).await;
}

// #[madsim::test]
// async fn test_snapshot_unreliable_recover_concurrent_partition_linearizable_3b() {
//     // Test: unreliable net, restarts, partitions, snapshots, linearizability checks (3B) ...
//     generic_test_linearizability("3B", 15, 7, true, true, true, Some(1000)).await;
// }
