use super::{tester::*, N_SHARDS};
use futures::future;
use log::*;
use madsim::{
    rand::{self, Rng},
    task,
    time::{self, Duration},
};
use std::{collections::HashMap, net::SocketAddr};

macro_rules! addrs {
    ($($addr:expr),* $(,)?) => {
        vec![$(SocketAddr::from(([0, 0, 0, $addr as u8], 0))),*]
    }
}

// helper macro to construct groups
macro_rules! groups {
    ($( $gid:expr => $addrs:expr ),* $(,)?) => {{
        let mut map = std::collections::HashMap::new();
        $(
            map.insert($gid, $addrs);
        )*
        map
    }}
}

#[madsim::test]
async fn basic_4a() {
    let nservers = 3;
    let t = Tester::new(nservers, false).await;
    let ck = t.make_client(&t.all());

    info!("Test: Basic leave/join ...");

    let mut cfa = vec![];
    cfa.push(ck.query().await);

    ck.check(&[]).await;

    let gid1 = 1;
    let addr1 = addrs![11, 12, 13];
    ck.join(groups!(gid1 => addr1.clone())).await;
    ck.check(&[gid1]).await;
    cfa.push(ck.query().await);

    let gid2 = 2;
    let addr2 = addrs![21, 22, 23];
    ck.join(groups!(gid2 => addr2.clone())).await;
    ck.check(&[gid1, gid2]).await;
    cfa.push(ck.query().await);

    let cfx = ck.query().await;
    let sa1 = cfx.groups[&gid1].as_slice();
    assert_eq!(sa1, addr1, "wrong servers for gid {}", gid1);
    let sa2 = cfx.groups[&gid2].as_slice();
    assert_eq!(sa2, addr2, "wrong servers for gid {}", gid2);

    ck.leave(&[gid1]).await;
    ck.check(&[gid2]).await;
    cfa.push(ck.query().await);

    ck.leave(&[gid2]).await;
    cfa.push(ck.query().await);

    info!("  ... Passed");

    info!("Test: Historical queries ...");

    for s in 0..nservers {
        t.shutdown_server(s);
        for cf in cfa.iter() {
            let c = ck.query_at(cf.num).await;
            assert_eq!(&c, cf);
        }
        t.start_server(s).await;
        t.connect_all();
    }

    info!("  ... Passed");

    info!("Test: Move ...");

    let gid3 = 503;
    let addr3 = addrs![31, 32, 33];
    ck.join(groups!(gid3 => addr3)).await;
    let gid4 = 504;
    let addr4 = addrs![41, 42, 43];
    ck.join(groups!(gid4 => addr4)).await;
    for i in 0..N_SHARDS {
        let cf = ck.query().await;
        let shard = if i < N_SHARDS / 2 { gid3 } else { gid4 };
        ck.move_(i, shard).await;
        if cf.shards[&i] != shard {
            let cf1 = ck.query().await;
            assert!(cf1.num > cf.num, "Move should increase Tester.Num");
        }
    }
    let cf2 = ck.query().await;
    for i in 0..N_SHARDS {
        let shard = if i < N_SHARDS / 2 { gid3 } else { gid4 };
        assert_eq!(cf2.shards[&i], shard, "shard {} wrong group", i);
    }
    ck.leave(&[gid3]).await;
    ck.leave(&[gid4]).await;

    info!("  ... Passed");

    info!("Test: Concurrent leave/join ...");

    let npara = 10;
    let gids: Vec<u64> = (0..npara).map(|i| i as u64 * 10 + 100).collect();
    let mut handles = vec![];
    for &gid in gids.iter() {
        let cka = t.make_client(&t.all());
        handles.push(task::spawn_local(async move {
            cka.join(groups!(gid + 1000 => addrs![gid + 1])).await;
            cka.join(groups!(gid => addrs![gid + 2])).await;
            cka.leave(&[gid + 1000]).await;
        }));
    }
    future::join_all(handles).await;
    ck.check(&gids).await;

    info!("  ... Passed");

    info!("Test: Minimal transfers after joins ...");

    let c1 = ck.query().await;
    for i in 0..5 {
        let gid = npara + 1 + i;
        ck.join(groups!(gid => addrs![gid + 1, gid + 2, gid + 2]))
            .await;
    }
    let c2 = ck.query().await;
    for i in 1..=npara {
        for j in 0..c1.shards.len() {
            assert!(
                c2.shards[&j] == i && c1.shards[&j] != i,
                "non-minimal transfer after Join()s"
            );
        }
    }

    info!("  ... Passed");

    info!("Test: Minimal transfers after leaves ...");

    for i in 0..5 {
        ck.leave(&[npara + 1 + i]).await;
    }
    let c3 = ck.query().await;
    for i in 1..=npara {
        for j in 0..c1.shards.len() {
            assert!(
                c2.shards[&j] == i && c3.shards[&j] != i,
                "non-minimal transfer after Leave()s"
            );
        }
    }

    info!("  ... Passed");

    t.end();
}

#[madsim::test]
async fn multi_4a() {
    let nservers = 3;
    let t = Tester::new(nservers, false).await;
    let ck = t.make_client(&t.all());

    info!("Test: Multi-group leave/join ...");

    let mut cfa = vec![];
    cfa.push(ck.query().await);

    ck.check(&[]).await;

    let gid1 = 1;
    let addr1 = addrs![11, 12, 13];
    let gid2 = 2;
    let addr2 = addrs![21, 22, 23];
    ck.join(groups!(gid1 => addr1.clone(), gid2 => addr2.clone()))
        .await;
    ck.check(&[gid1, gid2]).await;
    cfa.push(ck.query().await);

    let gid3 = 3;
    let addr3 = addrs![31, 32, 33];
    ck.join(groups!(gid3 => addr3.clone())).await;
    ck.check(&[gid1, gid2, gid3]).await;
    cfa.push(ck.query().await);

    let cfx = ck.query().await;
    let sa1 = cfx.groups[&gid1].as_slice();
    assert_eq!(sa1, addr1, "wrong servers for gid {}", gid1);
    let sa2 = cfx.groups[&gid2].as_slice();
    assert_eq!(sa2, addr2, "wrong servers for gid {}", gid2);
    let sa3 = cfx.groups[&gid3].as_slice();
    assert_eq!(sa3, addr3, "wrong servers for gid {}", gid3);

    ck.leave(&[gid1, gid3]).await;
    ck.check(&[gid2]).await;
    cfa.push(ck.query().await);

    let cfx = ck.query().await;
    let sa2 = cfx.groups[&gid2].as_slice();
    assert_eq!(sa2, addr2, "wrong servers for gid {}", gid2);

    ck.leave(&[gid2]).await;

    info!("  ... Passed");

    info!("Test: Concurrent multi leave/join ...");

    let npara = 10;
    let gids: Vec<u64> = (0..npara).map(|i| i as u64 + 1000).collect();
    let mut handles = vec![];
    for &gid in gids.iter() {
        let cka = t.make_client(&t.all());
        handles.push(task::spawn_local(async move {
            cka.join(groups!(
                gid => addrs![gid + 1, gid + 2, gid + 3],
                gid + 1000 => addrs![gid + 1000 + 1],
                gid + 2000 => addrs![gid + 2000 + 1],
            ))
            .await;
            cka.leave(&[gid + 1000, gid + 2000]).await;
        }));
    }
    future::join_all(handles).await;

    ck.check(&gids).await;

    info!("  ... Passed");

    info!("Test: Minimal transfers after multijoins ...");

    let c1 = ck.query().await;
    let mut m = HashMap::new();
    for i in 0..5 {
        let gid = npara + 1 + i;
        m.insert(gid, addrs![gid + 1, gid + 2]);
    }
    ck.join(m).await;
    let c2 = ck.query().await;
    for i in 1..=npara {
        for j in 0..c1.shards.len() {
            assert!(
                c2.shards[&j] == i && c1.shards[&j] != i,
                "non-minimal transfer after Join()s"
            );
        }
    }

    info!("  ... Passed");

    info!("Test: Minimal transfers after multileaves ...");

    let l: Vec<u64> = (0..5).map(|i| npara + 1 + i).collect();
    ck.leave(&l).await;
    let c3 = ck.query().await;
    for i in 1..=npara {
        for j in 0..c1.shards.len() {
            assert!(
                c2.shards[&j] == i && c3.shards[&j] != i,
                "non-minimal transfer after Leave()s"
            );
        }
    }

    info!("  ... Passed");

    info!("Test: Check Same config on servers ...");

    let leader = t.leader().expect("Leader not found");
    let c = ck.query().await; // Tester leader claims
    t.shutdown_server(leader);

    let mut attempts = 0;
    while t.leader().is_some() {
        attempts += 1;
        assert!(attempts < 3, "Leader not found");
        time::sleep(Duration::from_secs(1)).await;
    }

    let c1 = ck.query().await;
    assert_eq!(c, c1);

    info!("  ... Passed");

    t.end();
}
