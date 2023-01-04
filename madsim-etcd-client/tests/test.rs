#![cfg(madsim)]

use madsim::{runtime::Handle, time::sleep};
use madsim_etcd_client::{Client, PutOptions, SimServer};
use std::time::Duration;

#[madsim::test]
async fn kv() {
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
        let mut client = client.kv_client();
        // put kv
        client.put("foo", "bar", None).await.unwrap();
        // get kv
        let resp = client.get("foo", None).await.unwrap();
        let kv = &resp.kvs()[0];
        assert_eq!(kv.key(), b"foo");
        assert_eq!(kv.value(), b"bar");
        assert_eq!(kv.lease(), 0);
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
        let (mut keeper, _) = lease_client.keep_alive(lease.id()).await.unwrap();
        sleep(Duration::from_secs(45)).await;
        keeper.keep_alive().await.unwrap();
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
