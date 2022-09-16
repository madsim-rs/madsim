// #![cfg(madsim)]

use madsim::runtime::Handle;
use madsim_rdkafka::{
    admin::*, consumer::BaseConsumer, producer::BaseProducer, ClientConfig, SimBroker,
};
use std::net::SocketAddr;
use std::time::Duration;

#[madsim::test]
async fn test() {
    let handle = Handle::current();
    let broker_addr = "10.0.0.1:50051".parse::<SocketAddr>().unwrap();
    handle
        .create_node()
        .name("broker")
        .ip(broker_addr.ip())
        .build()
        .spawn(async move {
            SimBroker::default().serve(broker_addr).await.unwrap();
        });
    madsim::time::sleep(Duration::from_secs(1)).await;

    handle
        .create_node()
        .name("admin")
        .ip("10.0.0.2".parse().unwrap())
        .build()
        .spawn(async move {
            let admin = ClientConfig::new()
                .set("bootstrap.servers", broker_addr.to_string())
                .create::<AdminClient<_>>()
                .expect("failed to create admin client");
            admin
                .create_topics(
                    &[NewTopic::new("topic", 3, TopicReplication::Fixed(1))],
                    &AdminOptions::new(),
                )
                .await
                .expect("failed to create topic");
        })
        .await
        .unwrap();

    handle
        .create_node()
        .name("producer-1")
        .ip("10.0.1.1".parse().unwrap())
        .build()
        .spawn(async move {
            let producer = ClientConfig::new()
                .set("bootstrap.servers", broker_addr.to_string())
                .create::<BaseProducer>()
                .expect("failed to create producer");
        });

    handle
        .create_node()
        .name("consumer-1")
        .ip("10.0.2.1".parse().unwrap())
        .build()
        .spawn(async move {
            let consumer = ClientConfig::new()
                .set("bootstrap.servers", broker_addr.to_string())
                .create::<BaseConsumer>()
                .expect("failed to create consumer");
        });

    handle
        .create_node()
        .name("consumer-2")
        .ip("10.0.2.2".parse().unwrap())
        .build()
        .spawn(async move {
            let consumer = ClientConfig::new()
                .set("bootstrap.servers", broker_addr.to_string())
                .create::<BaseConsumer>()
                .expect("failed to create consumer");
        });
}
