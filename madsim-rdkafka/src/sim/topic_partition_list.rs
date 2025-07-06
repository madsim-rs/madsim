use crate::error::KafkaResult;

/// A Kafka offset.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Offset {
    /// Start consuming from the beginning of the partition.
    Beginning,
    /// Start consuming from the end of the partition.
    End,
    /// Start consuming from the stored offset.
    Stored,
    /// Offset not assigned or invalid.
    Invalid,
    /// A specific offset to consume from.
    ///
    /// Note that while the offset is a signed integer, negative offsets will be
    /// rejected when passed to librdkafka.
    Offset(i64),
    /// An offset relative to the end of the partition.
    ///
    /// Note that while the offset is a signed integer, negative offsets will
    /// be rejected when passed to librdkafka.
    OffsetTail(i64),
}

/// A structure to store and manipulate a list of topics and partitions with optional offsets.
#[derive(Debug, Clone)]
pub struct TopicPartitionList {
    pub(crate) list: Vec<Elem>,
}

impl Default for TopicPartitionList {
    fn default() -> Self {
        Self::new()
    }
}

impl TopicPartitionList {
    /// Creates a new empty list with default capacity.
    pub fn new() -> TopicPartitionList {
        TopicPartitionList::with_capacity(5)
    }

    /// Creates a new empty list with the specified capacity.
    pub fn with_capacity(capacity: usize) -> TopicPartitionList {
        TopicPartitionList {
            list: Vec::with_capacity(capacity),
        }
    }

    /// Returns the number of elements in the list.
    pub fn count(&self) -> usize {
        self.list.len()
    }

    /// Adds a topic and partition to the list.
    pub fn add_partition<'a>(
        &'a mut self,
        topic: &str,
        partition: i32,
    ) -> TopicPartitionListElem<'a> {
        self.list.push(Elem {
            topic: topic.into(),
            partition,
            offset: Offset::Invalid,
        });
        TopicPartitionListElem {
            e: self.list.last().unwrap(),
        }
    }

    /// Adds a topic and partition to the list, with the specified offset.
    pub fn add_partition_offset(
        &mut self,
        topic: &str,
        partition: i32,
        offset: Offset,
    ) -> KafkaResult<()> {
        self.list.push(Elem {
            topic: topic.into(),
            partition,
            offset,
        });
        Ok(())
    }

    /// Returns all the elements of the list.
    pub fn elements<'a>(&'a self) -> Vec<TopicPartitionListElem<'a>> {
        self.list
            .iter()
            .map(|e| TopicPartitionListElem { e })
            .collect()
    }

    /// Returns all the elements of the list that belong to the specified topic.
    pub fn elements_for_topic<'a>(&'a self, topic: &str) -> Vec<TopicPartitionListElem<'a>> {
        self.list
            .iter()
            .filter(|e| e.topic == topic)
            .map(|e| TopicPartitionListElem { e })
            .collect()
    }
}

/// One element of the topic partition list.
pub struct TopicPartitionListElem<'a> {
    e: &'a Elem,
}

#[derive(Debug, Clone)]
pub(crate) struct Elem {
    pub(crate) topic: String,
    pub(crate) partition: i32,
    pub(crate) offset: Offset,
}

impl TopicPartitionListElem<'_> {
    /// Returns the topic name.
    pub fn topic(&self) -> &str {
        &self.e.topic
    }

    /// Returns the partition number.
    pub fn partition(&self) -> i32 {
        self.e.partition
    }

    /// Returns the offset.
    pub fn offset(&self) -> Offset {
        self.e.offset
    }
}
