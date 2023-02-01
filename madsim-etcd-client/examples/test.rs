//! Test the behavior of a real etcd server.

#[cfg(not(madsim))]
#[tokio::main]
async fn main() {
    use madsim_etcd_client::*;

    let mut client = Client::connect(["127.0.0.1:2379"], None).await.unwrap();

    // // campaign_with_invalid_lease
    // let resp = client.campaign("leader", "1", 0x1234).await;
    // println!("{resp:?}");

    // revoke_invalid_lease
    // let resp = client.lease_revoke(0x1234).await;
    // println!("{resp:?}");

    // put_with_invalid_lease
    let resp = client
        .put("key", "value", Some(PutOptions::new().with_lease(1)))
        .await;
    println!("{resp:?}");
}

#[cfg(madsim)]
fn main() {}
