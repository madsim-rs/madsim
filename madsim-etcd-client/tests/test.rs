#![cfg(madsim)]

use madsim::{runtime::Handle, time::sleep};
use madsim_etcd_client::{Client, SimServer};
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
            client
                .kv_client()
                .put("foo", &b"bar\xFF\x01\x02"[..], None)
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
