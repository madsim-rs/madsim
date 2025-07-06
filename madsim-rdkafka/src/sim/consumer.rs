use futures_util::{Stream, StreamExt};
use madsim::net::Endpoint;
use serde::Deserialize;
use spin::Mutex;
use tracing::*;

use std::{
    collections::{HashSet, VecDeque},
    net::SocketAddr,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
    time::Duration,
};

use crate::{
    broker::FetchOptions,
    client::ClientContext,
    config::{FromClientConfig, FromClientConfigAndContext},
    error::{KafkaError, KafkaResult},
    message::{BorrowedMessage, OwnedMessage},
    metadata::Metadata,
    sim_broker::Request,
    topic_partition_list::Elem,
    util::Timeout,
    ClientConfig, Offset, TopicPartitionList,
};

/// Common trait for all consumers.
pub trait Consumer<C = DefaultConsumerContext>
where
    C: ConsumerContext,
{
}

/// Consumer-specific context.
pub trait ConsumerContext: ClientContext {}

/// An inert [`ConsumerContext`] that can be used when no customizations are needed.
#[derive(Clone, Debug, Default)]
pub struct DefaultConsumerContext;

impl ClientContext for DefaultConsumerContext {}
impl ConsumerContext for DefaultConsumerContext {}

/// A low-level consumer that requires manual polling.
///
/// This consumer must be periodically polled to make progress on rebalancing,
/// callbacks and to receive messages.
pub struct BaseConsumer<C = DefaultConsumerContext>
where
    C: ConsumerContext,
{
    _context: C,
    config: ConsumerConfig,
    ep: Endpoint,
    addr: SocketAddr,
    tpl: Mutex<TopicPartitionList>,
    msgs: Mutex<VecDeque<OwnedMessage>>,
}

#[async_trait::async_trait]
impl FromClientConfig for BaseConsumer {
    async fn from_config(config: &ClientConfig) -> KafkaResult<BaseConsumer> {
        BaseConsumer::from_config_and_context(config, DefaultConsumerContext).await
    }
}

/// Creates a new `BaseConsumer` starting from a `ClientConfig`.
#[async_trait::async_trait]
impl<C: ConsumerContext> FromClientConfigAndContext<C> for BaseConsumer<C> {
    async fn from_config_and_context(
        config: &ClientConfig,
        _context: C,
    ) -> KafkaResult<BaseConsumer<C>> {
        let config_json = serde_json::to_string(&config.conf_map)
            .map_err(|e| KafkaError::ClientCreation(e.to_string()))?;
        let config: ConsumerConfig = serde_json::from_str(&config_json)
            .map_err(|e| KafkaError::ClientCreation(e.to_string()))?;
        if config.enable_auto_commit {
            warn!("auto commit is not supported yet. consider set 'enable.auto.commit' = false");
        }
        if config.enable_partition_eof {
            warn!("partition eof is not supported yet");
        }
        if config.group_id.is_some() {
            warn!("group id is ignored");
        }
        let addr: SocketAddr = madsim::net::lookup_host(&config.bootstrap_servers)
            .await
            .map_err(|e| KafkaError::ClientCreation(e.to_string()))?
            .next()
            .ok_or_else(|| KafkaError::ClientCreation("invalid host or ip".into()))?;
        let p = BaseConsumer {
            _context,
            config,
            ep: Endpoint::bind("0.0.0.0:0")
                .await
                .map_err(|e| KafkaError::ClientCreation(e.to_string()))?,
            addr,
            tpl: Mutex::new(TopicPartitionList::new()),
            msgs: Mutex::new(VecDeque::new()),
        };
        Ok(p)
    }
}

impl<C> BaseConsumer<C>
where
    C: ConsumerContext,
{
    pub fn assign(&self, assignment: &TopicPartitionList) -> KafkaResult<()> {
        let mut tpl = assignment.clone();
        // auto offset reset
        for e in &mut tpl.list {
            self.reset_offset(e);
        }
        *self.tpl.lock() = tpl;
        Ok(())
    }

    pub fn incremental_assign(&self, assignment: &TopicPartitionList) -> KafkaResult<()> {
        let mut new_tpl = assignment.clone();

        for e in &mut new_tpl.list {
            self.reset_offset(e);
        }

        let mut current_tpl = self.tpl.lock();

        let existing_partitions: HashSet<_> = current_tpl
            .list
            .iter()
            .map(|elem| (elem.topic.to_string(), elem.partition))
            .collect();

        for new_elem in new_tpl.list {
            let partition_key = (new_elem.topic.to_string(), new_elem.partition);
            if !existing_partitions.contains(&partition_key) {
                // The partition is new, so add it to the current assignment.
                current_tpl.list.push(new_elem);
            }
            // If the partition already exists, we simply do nothing,
            // which makes the operation idempotent.
        }

        Ok(())
    }

    pub fn incremental_unassign(&self, unassignment: &TopicPartitionList) -> KafkaResult<()> {
        let mut current_tpl = self.tpl.lock();

        if current_tpl.list.is_empty() {
            return Ok(());
        }

        let partitions_to_remove: HashSet<_> = unassignment
            .list
            .iter()
            .map(|elem| (elem.topic.to_string(), elem.partition))
            .collect();

        if partitions_to_remove.is_empty() {
            return Ok(());
        }

        current_tpl.list.retain(|elem| {
            let partition_key = (elem.topic.to_string(), elem.partition);
            !partitions_to_remove.contains(&partition_key)
        });

        Ok(())
    }

    fn reset_offset(&self, e: &mut Elem) {
        if e.offset == Offset::Invalid {
            match self.config.auto_offset_reset {
                AutoOffsetResetStrategy::Latest => e.offset = Offset::End,
                AutoOffsetResetStrategy::Earliest => e.offset = Offset::Beginning,
                AutoOffsetResetStrategy::None => {}
            }
        }
    }

    /// Returns the low and high watermarks for a specific topic and partition.
    pub async fn fetch_watermarks(
        &self,
        topic: &str,
        partition: i32,
        _timeout: impl Into<Timeout>, // TODO: timeout
    ) -> KafkaResult<(i64, i64)> {
        let req = Request::FetchWatermarks {
            topic: topic.to_string(),
            partition,
        };
        let (tx, mut rx) = self.ep.connect1(self.addr).await?;
        tx.send(Box::new(req)).await?;
        *rx.recv().await?.downcast().unwrap()
    }

    pub async fn offsets_for_times(
        &self,
        timestamps: TopicPartitionList,
        _timeout: impl Into<Timeout>, // TODO: timeout
    ) -> KafkaResult<TopicPartitionList> {
        let req = Request::OffsetsForTimes { tpl: timestamps };
        let (tx, mut rx) = self.ep.connect1(self.addr).await?;
        tx.send(Box::new(req)).await?;
        *rx.recv().await?.downcast().unwrap()
    }

    pub async fn fetch_metadata(
        &self,
        topic: Option<&str>,
        _timeout: impl Into<Timeout>, // TODO: timeout
    ) -> KafkaResult<Metadata> {
        let req = Request::FetchMetadata {
            topic: topic.map(|s| s.to_string()),
        };
        let (tx, mut rx) = self.ep.connect1(self.addr).await?;
        tx.send(Box::new(req)).await?;
        *rx.recv().await?.downcast().unwrap()
    }
}

impl<C> BaseConsumer<C>
where
    C: ConsumerContext,
{
    /// Polls the consumer for new messages.
    pub async fn poll(
        &self,
        _timeout: impl Into<Timeout>, // TODO: timeout
    ) -> Option<KafkaResult<BorrowedMessage<'_>>> {
        self.poll_internal()
            .await
            .map(|res| res.map(|msg| msg.borrow()))
            .transpose()
    }

    async fn poll_internal(&self) -> KafkaResult<Option<OwnedMessage>> {
        // FIXME: concurrent call
        if self.msgs.lock().is_empty() {
            let tpl = self.tpl.lock().clone();
            if tpl.count() == 0 {
                return Ok(None);
            }
            let req = Request::Fetch {
                tpl,
                opts: FetchOptions {
                    fetch_max_bytes: self.config.fetch_max_bytes,
                    max_partition_fetch_bytes: self.config.max_partition_fetch_bytes,
                },
            };
            let (tx, mut rx) = self.ep.connect1(self.addr).await?;
            tx.send(Box::new(req)).await?;
            let rsp = *(rx.recv().await?)
                .downcast::<KafkaResult<(Vec<OwnedMessage>, TopicPartitionList)>>()
                .unwrap();
            let (msgs, tpl) = rsp?;
            if !msgs.is_empty() {
                debug!("fetched {} messages", msgs.len());
            }
            *self.msgs.lock() = VecDeque::from(msgs);
            *self.tpl.lock() = tpl;
        }
        Ok(self.msgs.lock().pop_front())
    }
}

/// A high-level consumer with a [`Stream`](futures::Stream) interface.
#[must_use = "Consumer polling thread will stop immediately if unused"]
pub struct StreamConsumer<C = DefaultConsumerContext>
where
    C: ConsumerContext,
{
    base: Arc<BaseConsumer<C>>,
    rx: async_channel::Receiver<KafkaResult<OwnedMessage>>,
    _task: madsim::task::FallibleTask<()>,
}

#[async_trait::async_trait]
impl FromClientConfig for StreamConsumer {
    async fn from_config(config: &ClientConfig) -> KafkaResult<StreamConsumer> {
        StreamConsumer::from_config_and_context(config, DefaultConsumerContext).await
    }
}

/// Creates a new `StreamConsumer` starting from a `ClientConfig`.
#[async_trait::async_trait]
impl<C: ConsumerContext> FromClientConfigAndContext<C> for StreamConsumer<C> {
    async fn from_config_and_context(
        config: &ClientConfig,
        context: C,
    ) -> KafkaResult<StreamConsumer<C>> {
        let base = Arc::new(BaseConsumer::from_config_and_context(config, context).await?);
        let consumer = base.clone();
        let (tx, rx) = async_channel::unbounded();
        // spawn a task to poll records periodically
        let _task = madsim::task::spawn(async move {
            loop {
                if let Some(res) = consumer.poll_internal().await.transpose() {
                    if tx.send(res).await.is_err() {
                        return;
                    }
                } else {
                    madsim::time::sleep(Duration::from_secs(1)).await;
                }
            }
        })
        .cancel_on_drop();
        Ok(Self { base, rx, _task })
    }
}

impl<C> StreamConsumer<C>
where
    C: ConsumerContext,
{
    pub fn assign(&self, assignment: &TopicPartitionList) -> KafkaResult<()> {
        self.base.assign(assignment)
    }

    pub fn incremental_assign(&self, assignment: &TopicPartitionList) -> KafkaResult<()> {
        self.base.incremental_assign(assignment)
    }

    pub fn incremental_unassign(&self, assignment: &TopicPartitionList) -> KafkaResult<()> {
        self.base.incremental_unassign(assignment)
    }

    pub async fn fetch_watermarks(
        &self,
        topic: &str,
        partition: i32,
        timeout: impl Into<Timeout>,
    ) -> KafkaResult<(i64, i64)> {
        self.base.fetch_watermarks(topic, partition, timeout).await
    }

    pub async fn offsets_for_times(
        &self,
        timestamps: TopicPartitionList,
        timeout: impl Into<Timeout>,
    ) -> KafkaResult<TopicPartitionList> {
        self.base.offsets_for_times(timestamps, timeout).await
    }

    pub async fn fetch_metadata(
        &self,
        topic: Option<&str>,
        timeout: impl Into<Timeout>,
    ) -> KafkaResult<Metadata> {
        self.base.fetch_metadata(topic, timeout).await
    }
}

impl<C> StreamConsumer<C>
where
    C: ConsumerContext,
{
    /// Constructs a stream that yields messages from this consumer.
    pub fn stream(&self) -> MessageStream<'_, C> {
        MessageStream {
            _consumer: self,
            rx: self.rx.clone(),
        }
    }
}

pub struct MessageStream<'a, C>
where
    C: ConsumerContext,
{
    _consumer: &'a StreamConsumer<C>,
    rx: async_channel::Receiver<KafkaResult<OwnedMessage>>,
}

impl<'a, C> Stream for MessageStream<'a, C>
where
    C: ConsumerContext,
{
    type Item = KafkaResult<BorrowedMessage<'a>>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.rx.poll_next_unpin(cx).map_ok(|msg| msg.borrow())
    }
}

/// Consumers configs.
///
/// <https://kafka.apache.org/documentation/#consumerconfigs>
#[derive(Debug, Default, Deserialize)]
struct ConsumerConfig {
    #[serde(rename = "bootstrap.servers")]
    bootstrap_servers: String,

    #[serde(rename = "group.id")]
    group_id: Option<String>,

    /// If true the consumer's offset will be periodically committed in the background.
    #[serde(
        rename = "enable.auto.commit",
        deserialize_with = "super::from_str",
        default = "default_enable_auto_commit"
    )]
    enable_auto_commit: bool,

    /// The maximum amount of data the server should return for a fetch request.
    #[serde(rename = "fetch.max.bytes", default = "default_fetch_max_bytes")]
    fetch_max_bytes: u32,

    /// The maximum amount of data per-partition the server will return.
    #[serde(
        rename = "max.partition.fetch.bytes",
        alias = "fetch.message.max.bytes",
        default = "default_max_partition_fetch_bytes"
    )]
    max_partition_fetch_bytes: u32,

    /// What to do when there is no initial offset in Kafka or if the current offset does not exist
    /// any more on the server (e.g. because that data has been deleted)
    #[serde(rename = "auto.offset.reset", default = "default_auto_offset_reset")]
    auto_offset_reset: AutoOffsetResetStrategy,

    /// Emit `PartitionEOF` event whenever the consumer reaches the end of a partition.
    #[serde(
        rename = "enable.partition.eof",
        deserialize_with = "super::from_str",
        default = "default_enable_partition_eof"
    )]
    enable_partition_eof: bool,
}

#[derive(Debug, Default, Deserialize)]
enum AutoOffsetResetStrategy {
    #[default]
    #[serde(rename = "latest", alias = "largest", alias = "end")]
    Latest,
    #[serde(rename = "earliest", alias = "smallest", alias = "beginning")]
    Earliest,
    #[serde(rename = "error")]
    None,
}

const fn default_enable_auto_commit() -> bool {
    true
}
const fn default_fetch_max_bytes() -> u32 {
    52428800
}
const fn default_max_partition_fetch_bytes() -> u32 {
    1048576
}
fn default_auto_offset_reset() -> AutoOffsetResetStrategy {
    AutoOffsetResetStrategy::Latest
}
const fn default_enable_partition_eof() -> bool {
    false
}
