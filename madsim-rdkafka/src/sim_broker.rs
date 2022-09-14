use crate::{
    broker::{Broker, Consumer, OwnedRecord},
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
                    Request::Consume { tpl } => {
                        let mut consumer = Consumer::new(tpl);
                        Box::new(service.lock().consume(&mut consumer))
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
    Consume { tpl: TopicPartitionList },
}
