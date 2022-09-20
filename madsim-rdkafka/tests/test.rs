// #![cfg(madsim)]

use madsim::runtime::Handle;
use madsim_rdkafka::{
    admin::*,
    consumer::{BaseConsumer, Consumer, StreamConsumer},
    producer::{BaseProducer, BaseRecord},
    ClientConfig, SimBroker, TopicPartitionList,
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
                .await
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
                .await
                .expect("failed to create producer");

            for i in 0..100 {
                let key = format!("1.{}", i);
                let payload = format!("message {}", i);
                let record = BaseRecord::to("topic").key(&key).payload(&payload);
                producer.send(record).expect("failed to send message");
                if i % 10 == 0 {
                    producer.poll().await;
                }
            }
        });

    handle
        .create_node()
        .name("producer-2")
        .ip("10.0.1.2".parse().unwrap())
        .build()
        .spawn(async move {
            let producer = ClientConfig::new()
                .set("bootstrap.servers", broker_addr.to_string())
                .create::<BaseProducer>()
                .await
                .expect("failed to create producer");

            for i in 0..100 {
                let key = format!("2.{}", i);
                let payload = format!("message {}", i);
                let record = BaseRecord::to("topic").key(&key).payload(&payload);
                producer.send(record).expect("failed to send message");
            }
        });

    let h1 = handle
        .create_node()
        .name("consumer-1")
        .ip("10.0.2.1".parse().unwrap())
        .build()
        .spawn(async move {
            let consumer = ClientConfig::new()
                .set("bootstrap.servers", broker_addr.to_string())
                .set("enable.auto.commit", "false")
                .create::<BaseConsumer>()
                .await
                .expect("failed to create consumer");

            let mut assignment = TopicPartitionList::new();
            assignment.add_partition("topic", 0);
            assignment.add_partition("topic", 1);
            consumer
                .assign(&assignment)
                .await
                .expect("failed to assign");

            let mut count = 0;
            // while count < 100 {
            for _ in 0..100 {
                let msg = match consumer.poll().await {
                    None => {
                        madsim::time::sleep(Duration::from_secs(1)).await;
                        continue;
                    }
                    Some(res) => res.unwrap(),
                };
                count += 1;
            }
        });

    let h2 = handle
        .create_node()
        .name("consumer-2")
        .ip("10.0.2.2".parse().unwrap())
        .build()
        .spawn(async move {
            let consumer = ClientConfig::new()
                .set("bootstrap.servers", broker_addr.to_string())
                .set("enable.auto.commit", "false")
                .create::<StreamConsumer>()
                .await
                .expect("failed to create consumer");

            let mut assignment = TopicPartitionList::new();
            assignment.add_partition("topic", 1);
            assignment.add_partition("topic", 2);
            consumer
                .assign(&assignment)
                .await
                .expect("failed to assign");
        });

    h1.await.unwrap();
    h2.await.unwrap();
}
