#![cfg(madsim)]

use futures_util::StreamExt;
use madsim::net::NetSim;
use madsim::runtime::Handle;
use madsim_rdkafka::{
    admin::*,
    consumer::{BaseConsumer, StreamConsumer},
    producer::{BaseProducer, BaseRecord},
    ClientConfig, Message, SimBroker, TopicPartitionList,
};
use std::{
    net::SocketAddr,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::Duration,
};

#[madsim::test]
async fn test() {
    let handle = Handle::current();
    let broker_addr = "10.0.0.1:50051".parse::<SocketAddr>().unwrap();
    NetSim::current().add_dns_record("broker", broker_addr.ip());

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
                .set("bootstrap.servers", "broker:50051")
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
                .set("bootstrap.servers", "broker:50051")
                .create::<BaseProducer>()
                .await
                .expect("failed to create producer");

            // generate an element every 0.1s
            for i in 1..=30 {
                let key = format!("1.{i}");
                let payload = [i as u8];
                let record = BaseRecord::to("topic").key(&key).payload(&payload);
                producer.send(record).expect("failed to send message");
                madsim::time::sleep(Duration::from_millis(100)).await;
                if i % 10 == 0 {
                    producer.flush(None).await.expect("failed to flush");
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
                .set("bootstrap.servers", "broker:50051")
                .create::<BaseProducer>()
                .await
                .expect("failed to create producer");

            // generate an element every 0.2s
            for i in 1..=30 {
                let key = format!("2.{i}");
                let payload = [i as u8];
                let record = BaseRecord::to("topic").key(&key).payload(&payload);
                producer.send(record).expect("failed to send message");
                madsim::time::sleep(Duration::from_millis(200)).await;
                if i % 10 == 0 {
                    producer.flush(None).await.expect("failed to flush");
                }
            }
        });

    let sum = Arc::new(AtomicUsize::new(0));
    let sum1 = sum.clone();
    let sum2 = sum.clone();

    handle
        .create_node()
        .name("consumer-1")
        .ip("10.0.2.1".parse().unwrap())
        .build()
        .spawn(async move {
            let consumer = ClientConfig::new()
                .set("bootstrap.servers", "broker:50051")
                .set("enable.auto.commit", "false")
                .set("auto.offset.reset", "earliest")
                .create::<BaseConsumer>()
                .await
                .expect("failed to create consumer");

            let mut assignment = TopicPartitionList::new();
            assignment.add_partition("topic", 0);
            assignment.add_partition("topic", 1);
            consumer.assign(&assignment).expect("failed to assign");

            loop {
                let msg = match consumer.poll().await {
                    None => {
                        madsim::time::sleep(Duration::from_millis(100)).await;
                        continue;
                    }
                    Some(res) => res.unwrap(),
                };
                let value = msg.payload().unwrap()[0];
                sum1.fetch_add(value as usize, Ordering::Relaxed);
            }
        });

    handle
        .create_node()
        .name("consumer-2")
        .ip("10.0.2.2".parse().unwrap())
        .build()
        .spawn(async move {
            let consumer = ClientConfig::new()
                .set("bootstrap.servers", "broker:50051")
                .set("enable.auto.commit", "false")
                .set("auto.offset.reset", "earliest")
                .create::<StreamConsumer>()
                .await
                .expect("failed to create consumer");

            let mut assignment = TopicPartitionList::new();
            assignment.add_partition("topic", 2);
            consumer.assign(&assignment).expect("failed to assign");

            let mut stream = consumer.stream();
            while let Some(msg) = stream.next().await {
                let value = msg.unwrap().payload().unwrap()[0];
                sum2.fetch_add(value as usize, Ordering::Relaxed);
            }
        });

    madsim::time::sleep(Duration::from_secs(10)).await;
    assert_eq!(sum.load(Ordering::Relaxed), (1..=30).sum::<usize>() * 2);
}
