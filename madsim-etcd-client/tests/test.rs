#![cfg(madsim)]

use madsim::{net::NetSim, runtime::Handle, time::sleep};
use madsim_etcd_client::{Client, ProclaimOptions, PutOptions, ResignOptions, SimServer};
use std::time::Duration;

#[madsim::test]
async fn kv() {
    let handle = Handle::current();
    let ip1 = "10.0.0.1".parse().unwrap();
    let ip2 = "10.0.0.2".parse().unwrap();
    let server = handle.create_node().name("server").ip(ip1).build();
    let client = handle.create_node().name("client").ip(ip2).build();
    NetSim::current().add_dns_record("etcd", ip1);

    server.spawn(async move {
        SimServer::builder()
            .serve("10.0.0.1:2379".parse().unwrap())
            .await
            .unwrap();
    });
    sleep(Duration::from_secs(1)).await;

    let task1 = client.spawn(async move {
        let client = Client::connect(["etcd:2379"], None).await.unwrap();
        let mut client = client.kv_client();
        // put kv
        client.put("foo", "bar", None).await.unwrap();
        // get kv
        let resp = client.get("foo", None).await.unwrap();
        let kv = &resp.kvs()[0];
        let revision = resp.header().expect("no header").revision();
        assert_eq!(kv.key(), b"foo");
        assert_eq!(kv.value(), b"bar");
        assert_eq!(kv.lease(), 0);
        assert_eq!(kv.create_revision(), revision);
        assert_eq!(kv.mod_revision(), revision);
        // put again
        client.put("foo", "gg", None).await.unwrap();
        // get again
        let resp = client.get("foo", None).await.unwrap();
        let kv = &resp.kvs()[0];
        assert_eq!(kv.value(), b"gg");
        assert_eq!(kv.create_revision(), revision);
        assert_eq!(
            kv.mod_revision(),
            resp.header().expect("no header").revision()
        );
    });
    task1.await.unwrap();
}

#[madsim::test]
async fn lease() {
    let handle = Handle::current();
    let ip1 = "10.0.0.1".parse().unwrap();
    let ip2 = "10.0.0.2".parse().unwrap();
    let server = handle.create_node().name("server").ip(ip1).build();
    let client = handle.create_node().name("client").ip(ip2).build();

    server.spawn(async move {
        SimServer::builder()
            .serve("10.0.0.1:2379".parse().unwrap())
            .await
            .unwrap();
    });
    sleep(Duration::from_secs(1)).await;

    let task1 = client.spawn(async move {
        let client = Client::connect(["10.0.0.1:2379"], None).await.unwrap();
        let mut kv_client = client.kv_client();
        let mut lease_client = client.lease_client();
        // grant lease
        let lease = lease_client.grant(60, None).await.unwrap();
        // put kv
        let opt = PutOptions::new().with_lease(lease.id());
        kv_client.put("foo", "bar", Some(opt)).await.unwrap();
        // get kv
        let resp = kv_client.get("foo", None).await.unwrap();
        assert_eq!(resp.kvs().len(), 1);
        assert_eq!(resp.kvs()[0].lease(), lease.id());
        // list leases
        let resp = client.lease_client().leases().await.unwrap();
        assert_eq!(resp.leases().len(), 1);
        assert_eq!(resp.leases()[0].id(), lease.id());

        // keep alive for 90s
        sleep(Duration::from_secs(45)).await;
        let (mut keeper, mut responses) = lease_client.keep_alive(lease.id()).await.unwrap();
        sleep(Duration::from_secs(45)).await;
        keeper.keep_alive().await.unwrap();
        let resp = responses.message().await.unwrap().unwrap();
        assert_eq!(resp.id(), lease.id());
        assert!(resp.ttl() <= 60 && resp.ttl() > 50);

        // get kv
        let resp = kv_client.get("foo", None).await.unwrap();
        assert_eq!(resp.kvs().len(), 1);

        // wait for lease expired
        sleep(Duration::from_secs(60)).await;
        // get kv
        let resp = kv_client.get("foo", None).await.unwrap();
        assert!(resp.kvs().is_empty());
    });
    task1.await.unwrap();
}

#[madsim::test]
async fn election() {
    // tracing_subscriber::fmt::init();

    let handle = Handle::current();
    let ip1 = "10.0.0.1".parse().unwrap();
    let ip2 = "10.0.0.2".parse().unwrap();
    let ip3 = "10.0.0.3".parse().unwrap();
    let ip4 = "10.0.0.4".parse().unwrap();
    let server = handle.create_node().name("server").ip(ip1).build();
    let client1 = handle.create_node().name("client1").ip(ip2).build();
    let client2 = handle.create_node().name("client2").ip(ip3).build();
    let client3 = handle.create_node().name("client3").ip(ip4).build();

    server.spawn(async move {
        SimServer::builder()
            .serve("10.0.0.1:2379".parse().unwrap())
            .await
            .unwrap();
    });
    sleep(Duration::from_secs(1)).await;

    // first leader
    let task1 = client1.spawn(async move {
        let client = Client::connect(["10.0.0.1:2379"], None).await.unwrap();
        let mut lease_client = client.lease_client();
        let mut client = client.election_client();
        // sleep for a while to make sure observer is ready
        sleep(Duration::from_secs(5)).await;
        // grant lease
        let lease = lease_client.grant(60, None).await.unwrap();
        // campaign to be leader
        let resp = client.campaign("leader", "1", lease.id()).await.unwrap();
        let leader_key = resp.leader().unwrap();
        assert_eq!(leader_key.name(), b"leader");
        assert_eq!(leader_key.lease(), lease.id());
        // get leader
        let resp = client.leader("leader").await.unwrap();
        assert_eq!(resp.kv().unwrap().value(), b"1");
        // proclaim
        let opt = ProclaimOptions::new().with_leader(leader_key.clone());
        client.proclaim("1.1", Some(opt.clone())).await.unwrap();
        let resp = client.leader("leader").await.unwrap();
        assert_eq!(resp.kv().unwrap().value(), b"1.1");
        // wait for 30s
        sleep(Duration::from_secs(30)).await;
        // revoke lease to release leadership
        lease_client.revoke(lease.id()).await.unwrap();
        // proclaim should fail
        client.proclaim("1.2", Some(opt.clone())).await.unwrap_err();
    });

    // second leader
    let task2 = client2.spawn(async move {
        let client = Client::connect(["10.0.0.1:2379"], None).await.unwrap();
        let mut lease_client = client.lease_client();
        let mut client = client.election_client();
        // sleep for a while to make sure client1 become leader
        sleep(Duration::from_secs(10)).await;
        // grant lease
        let lease = lease_client.grant(60, None).await.unwrap();
        // to be leader
        let resp = client.campaign("leader", "2", lease.id()).await.unwrap();
        let leader_key = resp.leader().unwrap();
        assert_eq!(leader_key.name(), b"leader");
        assert_eq!(leader_key.lease(), lease.id());
        // resign
        let opt = ResignOptions::new().with_leader(leader_key.clone());
        client.resign(Some(opt)).await.unwrap();
    });

    // observer
    let task3 = client3.spawn(async move {
        let client = Client::connect(["10.0.0.1:2379"], None).await.unwrap();
        let mut client = client.election_client();

        let mut leader_stream = client.observe("leader").await.unwrap();
        let resp = leader_stream.message().await.unwrap().unwrap();
        assert_eq!(resp.kv().unwrap().value(), b"1");
        let resp = leader_stream.message().await.unwrap().unwrap();
        assert_eq!(resp.kv().unwrap().value(), b"1.1");
        let resp = leader_stream.message().await.unwrap().unwrap();
        assert_eq!(resp.kv().unwrap().value(), b"2");
    });

    task1.await.unwrap();
    task2.await.unwrap();
    task3.await.unwrap();
}

#[madsim::test]
async fn maintenance() {
    let handle = Handle::current();
    let ip1 = "10.0.0.1".parse().unwrap();
    let ip2 = "10.0.0.2".parse().unwrap();
    let server = handle.create_node().name("server").ip(ip1).build();
    let client = handle.create_node().name("client").ip(ip2).build();

    server.spawn(async move {
        SimServer::builder()
            .serve("10.0.0.1:2379".parse().unwrap())
            .await
            .unwrap();
    });
    sleep(Duration::from_secs(1)).await;

    let task1 = client.spawn(async move {
        let client = Client::connect(["10.0.0.1:2379"], None).await.unwrap();
        let mut client = client.maintenance_client();
        // get status
        client.status().await.unwrap();
    });
    task1.await.unwrap();
}

#[madsim::test]
async fn load_dump() {
    let handle = Handle::current();
    let ip1 = "10.0.0.1".parse().unwrap();
    let ip2 = "10.0.0.2".parse().unwrap();
    let server = handle.create_node().name("server").ip(ip1).build();
    let client = handle.create_node().name("client").ip(ip2).build();

    server.spawn(async move {
        SimServer::builder()
            .serve("10.0.0.1:2379".parse().unwrap())
            .await
            .unwrap();
    });
    sleep(Duration::from_secs(1)).await;

    let dump = client
        .spawn(async move {
            let mut client = Client::connect(["10.0.0.1:2379"], None).await.unwrap();
            let lease = client.lease_client().grant(60, None).await.unwrap();
            let opt = PutOptions::new().with_lease(lease.id());
            client
                .kv_client()
                .put("foo", &b"bar\xFF\x01\x02"[..], Some(opt))
                .await
                .unwrap();
            client.dump().await.unwrap()
        })
        .await
        .unwrap();
    tracing::info!(%dump);

    server.spawn(async move {
        SimServer::builder()
            .load(dump)
            .serve("10.0.0.1:2380".parse().unwrap())
            .await
            .unwrap();
    });
    sleep(Duration::from_secs(1)).await;

    client
        .spawn(async move {
            let client = Client::connect(["10.0.0.1:2380"], None).await.unwrap();
            let resp = client.kv_client().get("foo", None).await.unwrap();
            assert_eq!(resp.kvs()[0].value(), b"bar\xFF\x01\x02");
        })
        .await
        .unwrap();
}
