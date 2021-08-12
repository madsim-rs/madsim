use super::{msg::Config, tester::*, N_SHARDS};
use futures::future;
use log::*;
use madsim::{
    rand::{self, Rng},
    task,
    time::{self, Duration},
};
use std::collections::HashMap;

// helper macro to construct groups
macro_rules! groups {
    ($( $gid:expr => [$($str:expr),*] ),*) => {{
        let mut map = std::collections::HashMap::new();
        $(
            map.insert($gid, vec![$($str.to_string()),*]);
        )*
        map
    }}
}

#[madsim::test]
async fn test_basic_4a() {
    let nservers = 3;
    let t = Tester::new(nservers, false).await;
    let ck = t.make_client(&t.all());

    info!("Test: Basic leave/join ...\n");

    let mut cfa = vec![];
    cfa.push(ck.query().await);

    ck.check(&[]).await;

    let gid1 = 1;
    ck.join(groups!(gid1 => ["x", "y", "z"])).await;
    ck.check(&[gid1]).await;
    cfa.push(ck.query().await);

    let gid2 = 2;
    ck.join(groups!(gid2 => ["a", "b", "c"])).await;
    ck.check(&[gid1, gid2]).await;
    cfa.push(ck.query().await);

    let cfx = ck.query().await;
    let sa1 = cfx.groups[&gid1].as_slice();
    assert_eq!(sa1, ["x", "y", "z"], "wrong servers for gid {}", gid1);
    let sa2 = cfx.groups[&gid2].as_slice();
    assert_eq!(sa2, ["a", "b", "c"], "wrong servers for gid {}", gid2);

    ck.leave(vec![gid1]).await;
    ck.check(&[gid2]).await;
    cfa.push(ck.query().await);

    ck.leave(vec![gid2]).await;
    cfa.push(ck.query().await);

    info!("  ... Passed\n");

    info!("Test: Historical queries ...\n");

    for s in 0..nservers {
        t.shutdown_server(s);
        for cf in cfa.iter() {
            let c = ck.query_at(cf.num).await;
            assert_eq!(&c, cf);
        }
        t.start_server(s).await;
        t.connect_all();
    }

    info!("  ... Passed\n");

    info!("Test: Move ...\n");

    let gid3 = 503;
    ck.join(groups!(gid3 => ["3a", "3b", "3c"])).await;
    let gid4 = 504;
    ck.join(groups!(gid4 => ["4a", "4b", "4c"])).await;
    for i in 0..N_SHARDS {
        let cf = ck.query().await;
        let shard = if i < N_SHARDS / 2 { gid3 } else { gid4 };
        ck.move_(i as u64, shard).await;
        if cf.shards[i] != shard {
            let cf1 = ck.query().await;
            assert!(cf1.num > cf.num, "Move should increase Tester.Num");
        }
    }
    let cf2 = ck.query().await;
    for i in 0..N_SHARDS {
        let shard = if i < N_SHARDS / 2 { gid3 } else { gid4 };
        assert_eq!(
            cf2.shards[i], shard,
            "expected shard {} on gid {} actually {}",
            i, shard, cf2.shards[i]
        );
    }
    ck.leave(vec![gid3]).await;
    ck.leave(vec![gid4]).await;

    info!("  ... Passed\n");

    info!("Test: Concurrent leave/join ...\n");

    let npara = 10;
    let gids: Vec<u64> = (0..npara).map(|i| i as u64 * 10 + 100).collect();
    let mut handles = vec![];
    for &gid in gids.iter() {
        let cka = t.make_client(&t.all());
        handles.push(task::spawn_local(async move {
            let sid1 = format!("s{}a", gid);
            let sid2 = format!("s{}b", gid);
            cka.join(groups!(gid + 1000 => [sid1])).await;
            cka.join(groups!(gid => [sid2])).await;
            cka.leave(vec![gid + 1000]).await;
        }));
    }
    future::join_all(handles).await;
    ck.check(&gids).await;

    info!("  ... Passed\n");

    info!("Test: Minimal transfers after joins ...\n");

    let c1 = ck.query().await;
    for i in 0..5 {
        let gid = npara + 1 + i;
        ck.join(groups!(gid => [format!("{}a", gid), format!("{}b", gid), format!("{}b", gid)]))
            .await;
    }
    let c2 = ck.query().await;
    for i in 1..=npara {
        assert_eq!(c1.shards.len(), c2.shards.len());
        for (&s1, &s2) in c1.shards.iter().zip(c2.shards.iter()) {
            if s2 == i && s1 != i {
                panic!("non-minimal transfer after Join()s");
            }
        }
    }

    info!("  ... Passed\n");

    info!("Test: Minimal transfers after leaves ...\n");

    for i in 0..5 {
        ck.leave(vec![npara + 1 + i]).await;
    }
    let c3 = ck.query().await;
    for i in 1..=npara {
        for j in 0..c1.shards.len() {
            if c2.shards[j] == i && c3.shards[j] != i {
                panic!("non-minimal transfer after Leave()s");
            }
        }
    }

    info!("  ... Passed\n");
}

#[madsim::test]
async fn test_multi_4a() {
    let nservers = 3;
    let t = Tester::new(nservers, false).await;
    let ck = t.make_client(&t.all());

    info!("Test: Multi-group leave/join ...\n");

    let mut cfa = vec![];
    cfa.push(ck.query().await);

    ck.check(&[]).await;

    let gid1 = 1;
    let gid2 = 2;
    ck.join(groups!(gid1 => ["x", "y", "z"], gid2 => ["a", "b", "c"]))
        .await;
    ck.check(&[gid1, gid2]).await;
    cfa.push(ck.query().await);

    let gid3 = 3;
    ck.join(groups!(gid3 => ["j", "k", "l"])).await;
    ck.check(&[gid1, gid2, gid3]).await;
    cfa.push(ck.query().await);

    let cfx = ck.query().await;
    let sa1 = cfx.groups[&gid1].as_slice();
    assert_eq!(sa1, ["x", "y", "z"], "wrong servers for gid {}", gid1);
    let sa2 = cfx.groups[&gid2].as_slice();
    assert_eq!(sa2, ["a", "b", "c"], "wrong servers for gid {}", gid2);
    let sa3 = cfx.groups[&gid3].as_slice();
    assert_eq!(sa3, ["j", "k", "l"], "wrong servers for gid {}", gid3);

    ck.leave(vec![gid1, gid3]).await;
    ck.check(&[gid2]).await;
    cfa.push(ck.query().await);

    let cfx = ck.query().await;
    let sa2 = cfx.groups[&gid2].as_slice();
    assert_eq!(sa2, ["a", "b", "c"], "wrong servers for gid {}", gid2);

    ck.leave(vec![gid2]).await;

    info!("  ... Passed\n");

    info!("Test: Concurrent multi leave/join ...\n");

    let npara = 10;
    let gids: Vec<u64> = (0..npara).map(|i| i as u64 + 1000).collect();
    let mut handles = vec![];
    for &gid in gids.iter() {
        let cka = t.make_client(&t.all());
        handles.push(task::spawn_local(async move {
            cka.join(groups!(
                gid => [
                    format!("{}a", gid),
                    format!("{}b", gid),
                    format!("{}c", gid)
                ],
                gid + 1000 => [format!("{}a", gid + 1000)],
                gid + 2000 => [format!("{}a", gid + 2000)]
            ))
            .await;
            cka.leave(vec![gid + 1000, gid + 2000]).await;
        }));
    }
    future::join_all(handles).await;

    ck.check(&gids).await;

    info!("  ... Passed\n");

    info!("Test: Minimal transfers after multijoins ...\n");

    let c1 = ck.query().await;
    let mut m = HashMap::new();
    for i in 0..5 {
        let gid = npara + 1 + i;
        m.insert(gid, vec![format!("{}a", gid), format!("{}b", gid)]);
    }
    ck.join(m).await;
    let c2 = ck.query().await;
    for i in 1..=npara {
        assert_eq!(c1.shards.len(), c2.shards.len());
        for (&s1, &s2) in c1.shards.iter().zip(c2.shards.iter()) {
            if s2 == i && s1 != i {
                panic!("non-minimal transfer after Join()s");
            }
        }
    }

    info!("  ... Passed\n");

    info!("Test: Minimal transfers after multileaves ...\n");

    let l: Vec<u64> = (0..5).map(|i| npara + 1 + i).collect();
    ck.leave(l).await;
    let c3 = ck.query().await;
    for i in 1..=npara {
        for j in 0..c1.shards.len() {
            if c2.shards[j] == i && c3.shards[j] != i {
                panic!("non-minimal transfer after Leave()s");
            }
        }
    }

    info!("  ... Passed\n");

    info!("Test: Check Same config on servers ...\n");

    let leader = t.leader().expect("Leader not found");
    let c = ck.query().await; // Tester leader claims
    t.shutdown_server(leader);

    let mut attempts = 0;
    while t.leader().is_ok() {
        attempts += 1;
        assert!(attempts < 3, "Leader not found");
        time::sleep(Duration::from_secs(1)).await;
    }

    let c1 = ck.query().await;
    assert_eq!(c, c1);

    info!("  ... Passed\n");
}
