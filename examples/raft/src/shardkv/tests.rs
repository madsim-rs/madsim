use super::tester::*;
use futures::future;
use log::*;
use madsim::{
    rand::{self, Rng},
    task,
    time::{self, Duration, Instant},
};
use std::{
    future::Future,
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc,
    },
};

/// test static 2-way sharding, without shard movement.
#[madsim::test]
async fn static_shards_4b() {
    info!("Test: static shards ...");
    let t = Tester::new(3, false, None).await;

    let ck = t.make_client();

    t.join(0).await;
    t.join(1).await;

    let n = 10;
    // ensure multiple shards
    let kvs = (0..n)
        .map(|i| (i.to_string(), rand_string(20)))
        .collect::<Vec<_>>();
    ck.put_kvs(&kvs).await;
    ck.check_kvs(&kvs).await;

    // make sure that the data really is sharded by
    // shutting down one shard and checking that some
    // Get()s don't succeed.
    t.shutdown_group(1);
    t.check_logs(); // forbid snapshots

    let ndone = Arc::new(AtomicUsize::new(0));
    let mut handles = vec![];
    for (k, v) in kvs.iter().cloned() {
        let ck = t.make_client();
        let ndone = ndone.clone();
        handles.push(task::spawn_local(async move {
            ck.check(k, v).await;
            ndone.fetch_add(1, Ordering::SeqCst);
        }));
    }
    // wait a bit, only about half the Gets should succeed.
    time::sleep(Duration::from_secs(2)).await;

    drop(handles);
    assert_eq!(
        ndone.load(Ordering::SeqCst),
        5,
        "expected 5 completions with one shard dead"
    );

    // bring the crashed shard/group back to life.
    t.start_group(1).await;
    ck.check_kvs(&kvs).await;

    t.end();
}

#[madsim::test]
async fn join_leave_4b() {
    info!("Test: join then leave ...");
    let t = Tester::new(3, false, None).await;

    let ck = t.make_client();
    t.join(0).await;

    let n = 10;
    let mut kvs = (0..n)
        .map(|i| (i.to_string(), rand_string(5)))
        .collect::<Vec<_>>();
    ck.put_kvs(&kvs).await;
    ck.check_kvs(&kvs).await;

    t.join(1).await;
    ck.check_append_kvs(&mut kvs, 5).await;

    t.leave(0).await;
    ck.check_append_kvs(&mut kvs, 5).await;

    // allow time for shards to transfer.
    time::sleep(Duration::from_secs(1)).await;

    t.check_logs();
    t.shutdown_group(0);

    ck.check_kvs(&kvs).await;

    t.end();
}

#[madsim::test]
async fn snapshot_4b() {
    info!("Test: snapshots, join, and leave ...");

    let t = Tester::new(3, false, None).await;
    let ck = t.make_client();

    t.join(0).await;

    let n = 30;
    let mut kvs = (0..n)
        .map(|i| (i.to_string(), rand_string(20)))
        .collect::<Vec<_>>();
    ck.put_kvs(&kvs).await;
    ck.check_kvs(&kvs).await;

    t.join(1).await;
    t.join(2).await;
    t.leave(0).await;
    ck.check_append_kvs(&mut kvs, 20).await;

    t.leave(1).await;
    t.join(0).await;
    ck.check_append_kvs(&mut kvs, 20).await;

    time::sleep(Duration::from_secs(1)).await;
    ck.check_kvs(&kvs).await;

    time::sleep(Duration::from_secs(1)).await;
    t.check_logs();

    for i in 0..3 {
        t.shutdown_group(i);
    }
    for i in 0..3 {
        t.start_group(i).await;
    }
    ck.check_kvs(&kvs).await;

    t.end();
}

#[madsim::test]
async fn miss_change_4b() {
    info!("Test: servers miss configuration changes...");

    let t = Tester::new(3, false, Some(1000)).await;
    let ck = t.make_client();

    t.join(0).await;

    let n = 10;
    let mut kvs = (0..n)
        .map(|i| (i.to_string(), rand_string(20)))
        .collect::<Vec<_>>();
    ck.put_kvs(&kvs).await;
    ck.check_kvs(&kvs).await;

    t.join(1).await;
    for i in 0..3 {
        t.shutdown_server(i, 0);
    }
    t.join(2).await;
    t.leave(0).await;
    t.leave(1).await;

    ck.check_append_kvs(&mut kvs, 20).await;

    t.join(1).await;
    ck.check_append_kvs(&mut kvs, 20).await;

    for i in 0..3 {
        t.start_server(i, 0).await;
    }
    ck.check_append_kvs(&mut kvs, 20).await;

    time::sleep(Duration::from_secs(2)).await;
    for i in 0..3 {
        t.shutdown_server(i, 1);
    }
    t.join(0).await;
    t.leave(2).await;
    ck.check_append_kvs(&mut kvs, 20).await;

    for i in 0..3 {
        t.start_server(i, 1).await;
    }
    ck.check_kvs(&kvs).await;

    t.end();
}

impl Tester {
    fn spawn_concurrent_append(
        &self,
        kvs: Vec<(String, String)>,
        append_len: usize,
        sleep_ms: u64,
    ) -> impl Future<Output = Vec<(String, String)>> {
        let done = Arc::new(AtomicBool::new(false));
        let mut handles = vec![];
        for (k, mut v) in kvs {
            let ck = self.make_client();
            let done = done.clone();
            handles.push(task::spawn_local(async move {
                while !done.load(Ordering::SeqCst) {
                    let s = rand_string(append_len);
                    v += &s;
                    ck.append(k.clone(), s).await;
                    time::sleep(Duration::from_millis(sleep_ms)).await;
                }
                (k, v)
            }));
        }
        async move {
            done.store(true, Ordering::SeqCst);
            future::join_all(handles).await
        }
    }
}

#[madsim::test]
async fn concurrent1_4b() {
    info!("Test: concurrent puts and configuration changes...");

    let t = Tester::new(3, false, Some(100)).await;
    let ck = t.make_client();

    t.join(0).await;

    let n = 10;
    let kvs = (0..n)
        .map(|i| (i.to_string(), rand_string(5)))
        .collect::<Vec<_>>();
    ck.put_kvs(&kvs).await;
    ck.check_kvs(&kvs).await;

    let kvs_fut = t.spawn_concurrent_append(kvs, 5, 10);

    time::sleep(Duration::from_millis(150)).await;
    t.join(1).await;
    time::sleep(Duration::from_millis(500)).await;
    t.join(2).await;
    time::sleep(Duration::from_millis(500)).await;
    t.leave(0).await;

    t.shutdown_group(0);
    time::sleep(Duration::from_millis(100)).await;
    t.shutdown_group(1);
    time::sleep(Duration::from_millis(100)).await;
    t.shutdown_group(2);

    t.leave(2).await;

    time::sleep(Duration::from_millis(100)).await;
    for i in 0..3 {
        t.start_group(i).await;
    }

    time::sleep(Duration::from_millis(100)).await;
    t.join(0).await;
    t.leave(1).await;
    time::sleep(Duration::from_millis(500)).await;
    t.join(1).await;

    time::sleep(Duration::from_secs(1)).await;
    let kvs = kvs_fut.await;

    ck.check_kvs(&kvs).await;

    t.end();
}

/// this tests the various sources from which a re-starting
/// group might need to fetch shard contents.
#[madsim::test]
async fn concurrent2_4b() {
    info!("Test: more concurrent puts and configuration changes...");

    let t = Tester::new(3, false, None).await;
    let ck = t.make_client();

    for i in 0..3 {
        t.join(i).await;
    }
    let n = 10;
    let kvs = (0..n)
        .map(|i| (i.to_string(), rand_string(1)))
        .collect::<Vec<_>>();
    ck.put_kvs(&kvs).await;

    let kvs_fut = t.spawn_concurrent_append(kvs, 1, 50);

    t.leave(0).await;
    t.leave(2).await;
    time::sleep(Duration::from_secs(3)).await;
    t.join(0).await;
    t.join(2).await;
    t.leave(1).await;
    time::sleep(Duration::from_secs(3)).await;
    t.join(1).await;
    t.leave(0).await;
    t.leave(2).await;
    time::sleep(Duration::from_secs(3)).await;

    t.shutdown_group(1);
    t.shutdown_group(2);
    time::sleep(Duration::from_secs(1)).await;
    t.start_group(1).await;
    t.start_group(2).await;

    time::sleep(Duration::from_secs(2)).await;
    let kvs = kvs_fut.await;

    ck.check_kvs(&kvs).await;

    t.end();
}

#[madsim::test]
async fn concurrent3_4b() {
    info!("Test: concurrent configuration change and restart...");

    let t = Tester::new(3, false, Some(300)).await;
    let ck = t.make_client();

    t.join(0).await;

    let n = 10;
    let kvs = (0..n)
        .map(|i| (i.to_string(), rand_string(1)))
        .collect::<Vec<_>>();
    ck.put_kvs(&kvs).await;

    let kvs_fut = t.spawn_concurrent_append(kvs, 1, 0);

    let mut random = rand::rng();
    let t0 = Instant::now();
    while t0.elapsed() < Duration::from_secs(12) {
        t.join(1).await;
        t.join(2).await;
        time::sleep(Duration::from_millis(random.gen_range(0..900))).await;
        for i in 0..3 {
            t.shutdown_group(i);
        }
        for i in 0..3 {
            t.start_group(i).await;
        }

        time::sleep(Duration::from_millis(random.gen_range(0..900))).await;
        t.leave(1).await;
        t.leave(2).await;
        time::sleep(Duration::from_millis(random.gen_range(0..900))).await;
    }

    time::sleep(Duration::from_secs(2)).await;
    let kvs = kvs_fut.await;

    ck.check_kvs(&kvs).await;

    t.end();
}

#[madsim::test]
async fn unreliable1_4b() {
    info!("Test: unreliable 1...");

    let t = Tester::new(3, true, Some(100)).await;
    let ck = t.make_client();

    t.join(0).await;

    let n = 10;
    let mut kvs = (0..n)
        .map(|i| (i.to_string(), rand_string(5)))
        .collect::<Vec<_>>();
    ck.put_kvs(&kvs).await;

    t.join(1).await;
    t.join(2).await;
    t.leave(0).await;
    ck.check_append_kvs(&mut kvs, 5).await;
    ck.check_append_kvs(&mut kvs, 5).await;

    t.join(0).await;
    t.leave(1).await;
    ck.check_kvs(&kvs).await;

    t.end();
}

#[madsim::test]
async fn unreliable2_4b() {
    info!("Test: unreliable 2...");

    let t = Tester::new(3, true, Some(100)).await;
    let ck = t.make_client();

    t.join(0).await;

    let n = 10;
    let kvs = (0..n)
        .map(|i| (i.to_string(), rand_string(5)))
        .collect::<Vec<_>>();
    ck.put_kvs(&kvs).await;

    let kvs_fut = t.spawn_concurrent_append(kvs, 5, 0);

    time::sleep(Duration::from_millis(150)).await;
    t.join(1).await;
    time::sleep(Duration::from_millis(500)).await;
    t.join(2).await;
    time::sleep(Duration::from_millis(500)).await;
    t.leave(0).await;
    time::sleep(Duration::from_millis(500)).await;
    t.leave(1).await;
    time::sleep(Duration::from_millis(500)).await;
    t.join(1).await;
    t.join(0).await;

    time::sleep(Duration::from_secs(2)).await;
    let kvs = kvs_fut.await;

    ck.check_kvs(&kvs).await;

    t.end();
}

#[ignore]
#[madsim::test]
async fn unreliable3_4b() {
    // TODO: linearizable
}

/// optional test to see whether servers are deleting
/// shards for which they are no longer responsible.
#[madsim::test]
async fn challenge1_delete_4b() {
    info!("Test: shard deletion (challenge 1) ...");

    // "1" means force snapshot after every log entry.
    let t = Tester::new(3, false, Some(1)).await;
    let ck = t.make_client();

    t.join(0).await;

    // 30,000 bytes of total values.
    let n = 30;
    let kvs = (0..n)
        .map(|i| (i.to_string(), rand_string(1000)))
        .collect::<Vec<_>>();
    ck.put_kvs(&kvs).await;
    ck.check_kvs(&kvs[..3]).await;

    for _iters in 0..2 {
        t.join(1).await;
        t.leave(0).await;
        t.join(2).await;

        time::sleep(Duration::from_secs(3)).await;
        ck.check_kvs(&kvs[..3]).await;
        t.leave(1).await;
        t.join(0).await;
        t.leave(2).await;

        time::sleep(Duration::from_secs(3)).await;
        ck.check_kvs(&kvs[..3]).await;
    }

    t.join(1).await;
    t.join(2).await;
    for _ in 0..3 {
        time::sleep(Duration::from_secs(1)).await;
        ck.check_kvs(&kvs[..3]).await;
    }

    let total = t.total_size();
    // 27 keys should be stored once.
    // 3 keys should also be stored in client dup tables.
    // everything on 3 replicas.
    // plus slop.
    let expected = 3 * (((n - 3) * 1000) + 2 * 3 * 1000 + 6000);
    assert!(
        total <= expected,
        "snapshot + persisted Raft state are too big: {} > {}",
        total,
        expected
    );

    ck.check_kvs(&kvs).await;

    t.end();
}

/// optional test to see whether servers can handle
/// shards that are not affected by a config change
/// while the config change is underway
#[madsim::test]
async fn challenge2_unaffected_4b() {
    info!("Test: unaffected shard access (challenge 2) ...");

    let t = Tester::new(3, true, Some(100)).await;
    let ck = t.make_client();

    // JOIN 100
    t.join(0).await;

    // Do a bunch of puts to keys in all shards
    let n = 10;
    let mut kvs = (0..n)
        .map(|i| (i.to_string(), "100".into()))
        .collect::<Vec<_>>();
    ck.put_kvs(&kvs).await;

    // JOIN 101
    t.join(1).await;

    // QUERY to find shards now owned by 101
    let owned = t.query_shards_of(1).await;

    // Wait for migration to new config to complete, and for clients to
    // start using this updated config. Gets to any key k such that
    // owned[shard(k)] == true should now be served by group 101.
    time::sleep(Duration::from_secs(1)).await;
    for (i, (k, v)) in kvs.iter_mut().enumerate() {
        if owned.contains(&i) {
            *v = "101".into();
            ck.put(k.clone(), "101".into()).await;
        }
    }

    // KILL 100
    t.shutdown_group(0);

    // LEAVE 100
    // 101 doesn't get a chance to migrate things previously owned by 100
    t.leave(0).await;

    // Wait to make sure clients see new config
    time::sleep(Duration::from_secs(1)).await;

    // And finally: check that gets/puts for 101-owned keys still complete
    for (i, (k, v)) in kvs.iter().enumerate() {
        let shard = super::client::key2shard(k);
        if owned.contains(&i) {
            ck.check(k.clone(), v.clone()).await;
            ck.put(k.clone(), v.clone() + "-1").await;
            ck.check(k.clone(), v.clone() + "-1").await;
        }
    }

    t.end();
}

/// optional test to see whether servers can handle operations on shards that
/// have been received as a part of a config migration when the entire migration
/// has not yet completed.
#[madsim::test]
async fn challenge2_partial_4b() {
    info!("Test: partial migration shard access (challenge 2) ...");

    let t = Tester::new(3, true, Some(100)).await;
    let ck = t.make_client();

    // JOIN 100 + 101 + 102
    t.joins(&[0, 1, 2]).await;

    // Give the implementation some time to reconfigure
    time::sleep(Duration::from_secs(1)).await;

    // Do a bunch of puts to keys in all shards
    let n = 10;
    let mut kvs = (0..n)
        .map(|i| (i.to_string(), "100".into()))
        .collect::<Vec<_>>();
    ck.put_kvs(&kvs).await;

    // QUERY to find shards now owned by 102
    let owned = t.query_shards_of(2).await;

    // KILL 100
    t.shutdown_group(0);

    // LEAVE 100 + 102
    // 101 can get old shards from 102, but not from 100. 101 should start
    // serving shards that used to belong to 102 as soon as possible
    t.leaves(&[0, 2]).await;

    // Give the implementation some time to start reconfiguration
    // And to migrate 102 -> 101
    time::sleep(Duration::from_secs(1)).await;

    // And finally: check that gets/puts for 101-owned keys now complete
    for (i, (k, v)) in kvs.iter().enumerate() {
        let shard = super::client::key2shard(k);
        if owned.contains(&i) {
            ck.check(k.clone(), v.clone()).await;
            ck.put(k.clone(), v.clone() + "-2").await;
            ck.check(k.clone(), v.clone() + "-2").await;
        }
    }

    t.end();
}
