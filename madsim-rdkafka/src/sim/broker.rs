//! A simulated Kafka broker.

use crate::{
    error::{KafkaError as Error, KafkaResult as Result, RDKafkaErrorCode as ErrorCode},
    message::{OwnedMessage, Timestamp},
    metadata::{Metadata, MetadataPartition, MetadataTopic},
    Message, Offset, TopicPartitionList,
};
use std::collections::HashMap;
use tracing::*;

#[derive(Debug, Default)]
pub struct Broker {
    topics: HashMap<String, Topic>,
}

#[derive(Debug)]
struct Topic {
    name: String,
    partitions: Vec<Partition>,
    last_partition: usize,
}

#[derive(Debug)]
struct Partition {
    id: i32,
    log_end_offset: i64,
    low_watermark: i64,
    high_watermark: i64,
    msgs: Vec<OwnedMessage>,
}

impl Partition {
    /// Creates a new partition.
    fn new(id: i32) -> Self {
        Partition {
            id,
            log_end_offset: 0,
            low_watermark: 0,
            high_watermark: 0,
            msgs: vec![],
        }
    }

    /// Looks up the offset by timestamp.
    ///
    /// The returned offset is the earliest offset whose timestamp
    /// is greater than or equal to the given timestamp.
    fn offset_for_time(&self, timestamp: i64) -> Option<i64> {
        let idx = self
            .msgs
            .partition_point(|msg| match msg.timestamp() {
                Timestamp::NotAvailable => 0,
                Timestamp::CreateTime(t) => t,
                Timestamp::LogAppendTime(t) => t,
            } < timestamp);
        self.msgs.get(idx).map(|msg| msg.offset())
    }
}

impl Broker {
    /// Creates a new topic.
    pub fn create_topic(&mut self, name: String, partitions: usize) -> Result<()> {
        debug!(?name, partitions, "create_topic");
        self.topics
            .insert(name.clone(), Topic::new(name, partitions));
        Ok(())
    }

    /// Produces messages.
    pub fn produce(&mut self, messages: Vec<OwnedMessage>) -> Result<()> {
        debug!("produce {} messages", messages.len());
        for msg in messages {
            self.produce_one(msg)?;
        }
        Ok(())
    }

    /// Produces a message.
    fn produce_one(&mut self, mut msg: OwnedMessage) -> Result<()> {
        let topic = self
            .topics
            .get_mut(&msg.topic)
            .ok_or(Error::MessageProduction(ErrorCode::UnknownTopic))?;

        let partition_idx = topic.last_partition;
        topic.last_partition += 1;
        if topic.last_partition >= topic.partitions.len() {
            topic.last_partition = 0;
        }

        let partition = &mut topic.partitions[partition_idx];
        msg.partition = partition_idx as _;
        msg.offset = partition.log_end_offset;

        trace!(?msg, "produce");
        partition.msgs.push(msg);
        partition.log_end_offset += 1;
        partition.high_watermark = partition.log_end_offset;
        Ok(())
    }

    /// Fetch records.
    pub fn fetch(
        &self,
        tpl: &mut TopicPartitionList,
        opts: FetchOptions,
    ) -> Result<Vec<OwnedMessage>> {
        debug!(list = ?tpl, "fetch");
        let mut rets = vec![];
        let mut total_bytes = 0;
        for e in &mut tpl.list {
            let partition = self
                .get_partition(&e.topic, e.partition)
                .map_err(Error::MessageConsumption)?;
            let msgs = &partition.msgs;
            if msgs.is_empty() {
                continue;
            }
            let start_idx = match e.offset {
                Offset::Beginning => 0,
                Offset::End => msgs.len() - 1,
                Offset::Stored => panic!("stored offset is not available"),
                Offset::Invalid => return Err(Error::MessageConsumption(ErrorCode::NoOffset)),
                Offset::Offset(offset) => msgs.partition_point(|msg| msg.offset() < offset),
                Offset::OffsetTail(_) => todo!("offset tail"),
            };
            let mut total_bytes_in_partition = 0;
            for msg in msgs.iter().skip(start_idx) {
                let size = msg.size();
                if msg.offset() >= partition.high_watermark {
                    continue;
                }
                if total_bytes + size > opts.fetch_max_bytes as usize
                    || total_bytes_in_partition + size > opts.max_partition_fetch_bytes as usize
                {
                    return Ok(rets);
                }
                e.offset = Offset::Offset(msg.offset() + 1);
                rets.push(msg.clone());
                total_bytes += size;
                total_bytes_in_partition += size;
            }
        }
        Ok(rets)
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

    /// Returns the low and high watermarks for a specific topic and partition.
    pub fn fetch_watermarks(&self, topic: &str, partition: i32) -> Result<(i64, i64)> {
        let partition = self
            .get_partition(topic, partition)
            .map_err(Error::OffsetFetch)?;
        Ok((partition.low_watermark, partition.high_watermark))
    }

    /// Looks up the offsets for the specified partitions by timestamp.
    pub fn offsets_for_times(&self, tpl: &TopicPartitionList) -> Result<TopicPartitionList> {
        let mut ret = TopicPartitionList::with_capacity(tpl.count());
        for e in &tpl.list {
            let partition = self
                .get_partition(&e.topic, e.partition)
                .map_err(Error::OffsetFetch)?;
            let timestamp = match e.offset {
                Offset::Offset(ts) => ts,
                _ => return Err(Error::OffsetFetch(ErrorCode::InvalidTimestamp)),
            };
            let offset = partition
                .offset_for_time(timestamp)
                .map_or(Offset::Invalid, Offset::Offset);
            ret.add_partition_offset(&e.topic, e.partition, offset)
                .unwrap();
        }
        Ok(ret)
    }

    fn get_partition(
        &self,
        topic: &str,
        partition: i32,
    ) -> std::result::Result<&Partition, ErrorCode> {
        let topic = self.topics.get(topic).ok_or(ErrorCode::UnknownTopic)?;
        let partition = &topic
            .partitions
            .get(partition as usize)
            .ok_or(ErrorCode::UnknownPartition)?;
        Ok(partition)
    }
}

impl Topic {
    /// Create a new [`Topic`].
    fn new(name: String, partitions: usize) -> Self {
        Topic {
            name,
            partitions: (0..partitions).map(|id| Partition::new(id as _)).collect(),
            last_partition: 0,
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

#[derive(Debug)]
pub struct FetchOptions {
    /// The maximum amount of data per-partition the server will return.
    ///
    /// Default: 1048576 (1 mebibyte)
    pub max_partition_fetch_bytes: u32,

    /// The maximum amount of data the server should return for a fetch request.
    ///
    /// Default: 52428800 (50 mebibytes)
    pub fetch_max_bytes: u32,
}

impl Default for FetchOptions {
    fn default() -> Self {
        Self {
            max_partition_fetch_bytes: 1048576,
            fetch_max_bytes: 52428800,
        }
    }
}
