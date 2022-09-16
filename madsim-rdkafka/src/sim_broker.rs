use crate::{
    broker::{Broker, Consumer, OwnedRecord},
    metadata::Metadata,
    TopicPartitionList,
};
use madsim::net::{Endpoint, Payload};
use spin::Mutex;
use std::{io::Result, net::SocketAddr, sync::Arc};

#[derive(Default)]
pub struct SimBroker {}

impl SimBroker {
    pub async fn serve(self, addr: SocketAddr) -> Result<()> {
        let ep = Endpoint::bind(addr).await?;
        let service = Arc::new(Mutex::new(Broker::default()));
        loop {
            let (tx, mut rx, _) = ep.accept1().await?;
            let service = service.clone();
            madsim::task::spawn(async move {
                let request = *rx.recv().await?.downcast::<Request>().unwrap();
                let response: Payload = match request {
                    Request::CreateTopic { name, partitions } => {
                        Box::new(service.lock().create_topic(name, partitions))
                    }
                    Request::Produce { record } => Box::new(service.lock().produce(record)),
                    Request::Fetch { tpl } => {
                        let mut consumer = Consumer::new(tpl);
                        Box::new(service.lock().fetch(&mut consumer))
                    }
                    Request::FetchMetadata { topic } => Box::new(match topic {
                        Some(topic) => service
                            .lock()
                            .metadata_of_topic(&topic)
                            .map(|m| Metadata { topics: vec![m] }),
                        None => service.lock().metadata(),
                    }),
                    Request::FetchWatermarks { topic, partition } => {
                        Box::new(service.lock().fetch_watermarks(&topic, partition))
                    }
                    Request::OffsetsForTimes { tpl } => {
                        Box::new(service.lock().offsets_for_times(&tpl))
                    }
                };
                tx.send(response).await?;
                Ok(()) as Result<()>
            });
        }
    }
}

/// Request to `SimBroker`.
#[derive(Debug)]
pub enum Request {
    CreateTopic { name: String, partitions: usize },
    Produce { record: OwnedRecord },
    Fetch { tpl: TopicPartitionList },
    FetchMetadata { topic: Option<String> },
    FetchWatermarks { topic: String, partition: i32 },
    OffsetsForTimes { tpl: TopicPartitionList },
}
