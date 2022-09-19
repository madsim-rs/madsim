use std::net::SocketAddr;

use futures_util::Stream;
use madsim::net::Endpoint;
use serde::Deserialize;

use crate::{
    client::ClientContext,
    config::{FromClientConfig, FromClientConfigAndContext},
    error::{KafkaError, KafkaResult},
    message::BorrowedMessage,
    metadata::Metadata,
    util::Timeout,
    ClientConfig, TopicPartitionList,
};

/// Common trait for all consumers.
#[async_trait::async_trait]
pub trait Consumer<C = DefaultConsumerContext>
where
    C: ConsumerContext,
{
    /// Manually assigns topics and partitions to the consumer. If used,
    /// automatic consumer rebalance won't be activated.
    async fn assign(&self, assignment: &TopicPartitionList) -> KafkaResult<()>;

    /// Returns the low and high watermarks for a specific topic and partition.
    async fn fetch_watermarks<T>(
        &self,
        topic: &str,
        partition: i32,
        timeout: T,
    ) -> KafkaResult<(i64, i64)>
    where
        T: Into<Timeout>,
        Self: Sized;

    /// Looks up the offsets for the specified partitions by timestamp.
    ///
    /// The returned offset for each partition is the earliest offset whose timestamp
    /// is greater than or equal to the given timestamp in the corresponding partition.
    async fn offsets_for_times<T>(
        &self,
        timestamps: TopicPartitionList,
        timeout: T,
    ) -> KafkaResult<TopicPartitionList>
    where
        T: Into<Timeout>,
        Self: Sized;

    /// Returns the metadata information for the specified topic, or for all
    /// topics in the cluster if no topic is specified.
    async fn fetch_metadata<T>(&self, topic: Option<&str>, timeout: T) -> KafkaResult<Metadata>
    where
        T: Into<Timeout>,
        Self: Sized;
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
    context: C,
    config: ConsumerConfig,
    ep: Endpoint,
    addr: SocketAddr,
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
        context: C,
    ) -> KafkaResult<BaseConsumer<C>> {
        let config_json = serde_json::to_string(&config.conf_map)
            .map_err(|e| KafkaError::ClientCreation(e.to_string()))?;
        let config: ConsumerConfig = serde_json::from_str(&config_json)
            .map_err(|e| KafkaError::ClientCreation(e.to_string()))?;
        let addr = config
            .bootstrap_servers
            .parse::<SocketAddr>()
            .map_err(|e| KafkaError::ClientCreation(e.to_string()))?;
        let mut p = BaseConsumer {
            context,
            config,
            ep: Endpoint::bind("0.0.0.0:0")
                .await
                .map_err(|e| KafkaError::ClientCreation(e.to_string()))?,
            addr,
        };
        Ok(p)
    }
}

impl<C> BaseConsumer<C>
where
    C: ConsumerContext,
{
    async fn assign(&self, assignment: &TopicPartitionList) -> KafkaResult<()> {
        todo!()
    }

    async fn fetch_watermarks(&self, topic: &str, partition: i32) -> KafkaResult<(i64, i64)> {
        todo!()
    }

    async fn offsets_for_times(
        &self,
        timestamps: TopicPartitionList,
    ) -> KafkaResult<TopicPartitionList> {
        todo!()
    }

    async fn fetch_metadata(&self, topic: Option<&str>) -> KafkaResult<Metadata> {
        todo!()
    }
}

/// Creates a new `StreamConsumer` starting from a `ClientConfig`.
#[async_trait::async_trait]
impl<C: ConsumerContext> FromClientConfigAndContext<C> for StreamConsumer<C> {
    async fn from_config_and_context(
        config: &ClientConfig,
        context: C,
    ) -> KafkaResult<StreamConsumer<C>> {
        todo!()
    }
}

/// A high-level consumer with a [`Stream`](futures::Stream) interface.
#[must_use = "Consumer polling thread will stop immediately if unused"]
pub struct StreamConsumer<C = DefaultConsumerContext>
where
    C: ConsumerContext,
{
    _base: BaseConsumer<C>,
    // _runtime: PhantomData<R>,
}

impl StreamConsumer {
    /// Constructs a stream that yields messages from this consumer.
    pub fn stream(&self) -> MessageStream<'_> {
        todo!()
    }
}

pub struct MessageStream<'a> {
    _runtime: &'a (),
}

impl<'a> Stream for MessageStream<'a> {
    type Item = KafkaResult<BorrowedMessage<'a>>;

    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        todo!()
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
    #[serde(rename = "enable.auto.commit", default = "default_enable_auto_commit")]
    enable_auto_commit: bool,

    /// The maximum amount of data the server should return for a fetch request.
    #[serde(rename = "fetch.max.bytes", default = "default_fetch_max_bytes")]
    fetch_max_bytes: u32,

    /// What to do when there is no initial offset in Kafka or if the current offset does not exist
    /// any more on the server (e.g. because that data has been deleted)
    #[serde(rename = "auto.offset.reset", default = "default_auto_offset_reset")]
    auto_offset_reset: String,

    /// Emit `PartitionEOF` event whenever the consumer reaches the end of a partition.
    #[serde(
        rename = "enable.partition.eof",
        default = "default_enable_partition_eof"
    )]
    enable_partition_eof: bool,
}

const fn default_enable_auto_commit() -> bool {
    true
}
const fn default_fetch_max_bytes() -> u32 {
    52428800
}
fn default_auto_offset_reset() -> String {
    "latest".into()
}
const fn default_enable_partition_eof() -> bool {
    false
}
