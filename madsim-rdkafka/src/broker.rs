//! A simulated Kafka broker.

use crate::{
    error::{KafkaError as Error, KafkaResult as Result, RDKafkaErrorCode as ErrorCode},
    message::OwnedMessage,
    metadata::{Metadata, MetadataPartition, MetadataTopic},
};
use std::collections::HashMap;
use tokio::sync::mpsc;

#[derive(Debug, Default)]
pub struct Builder {}

impl Builder {
    fn build(self) -> Broker {
        todo!()
    }
}

pub struct Broker {
    topics: HashMap<String, Topic>,
}

struct Topic {
    name: String,
    partitions: Vec<Partition>,
}

struct Partition {
    id: i32,
    watermark_lo: i64,
    watermark_hi: i64,
    consumer: mpsc::Sender<OwnedMessage>,
}

impl Partition {
    fn new(id: i32) -> Self {
        todo!()
    }
}

impl Broker {
    pub fn create_topic(&mut self, name: String, partitions: usize) -> Result<()> {
        self.topics
            .insert(name.clone(), Topic::new(name, partitions));
        Ok(())
    }

    /// Creates a producer of the given topic.
    pub fn create_producer(&mut self, topic: &str) -> Result<()> {
        todo!()
    }

    /// Creates a consumer of the given topic.
    pub fn create_consumer(&mut self, topic: &str) -> Result<()> {
        todo!()
    }

    /// Returns the metadata of this cluster.
    pub fn metadata(&self) -> Result<Metadata> {
        let topics = self.topics.values().map(|t| t.metadata()).collect();
        Ok(Metadata { topics })
    }

    /// Returns the metadata of the given topic.
    pub fn metadata_of_topic(&self, topic: &str) -> Result<MetadataTopic> {
        let topic = self
            .topics
            .get(topic)
            .ok_or(Error::MetadataFetch(ErrorCode::UnknownTopic))?;
        Ok(topic.metadata())
    }
}

impl Topic {
    /// Create a new [`Topic`].
    fn new(name: String, partitions: usize) -> Self {
        Topic {
            name,
            partitions: (0..partitions).map(|id| Partition::new(id as _)).collect(),
        }
    }

    /// Returns the metadata of this topic.
    fn metadata(&self) -> MetadataTopic {
        MetadataTopic {
            name: self.name.clone(),
            partitions: self
                .partitions
                .iter()
                .map(|p| MetadataPartition { id: p.id })
                .collect(),
        }
    }
}
