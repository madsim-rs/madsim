use std::net::SocketAddr;

use madsim::net::Endpoint;
use serde::Deserialize;
use spin::Mutex;
use tracing::*;

use crate::{
    broker::OwnedRecord,
    client::ClientContext,
    config::{FromClientConfig, FromClientConfigAndContext},
    error::{KafkaError, KafkaResult},
    message::{OwnedHeaders, ToBytes},
    sim_broker::Request,
    util::Timeout,
    ClientConfig,
};

/// A record for the [`BaseProducer`] and [`ThreadedProducer`].
#[derive(Debug)]
pub struct BaseRecord<'a, K: ToBytes + ?Sized = (), P: ToBytes + ?Sized = ()> {
    /// Required destination topic.
    pub topic: &'a str,
    /// Optional destination partition.
    pub partition: Option<i32>,
    /// Optional payload.
    pub payload: Option<&'a P>,
    /// Optional key.
    pub key: Option<&'a K>,
    /// Optional timestamp.
    ///
    /// Note that Kafka represents timestamps as the number of milliseconds
    /// since the Unix epoch.
    pub timestamp: Option<i64>,
    /// Optional message headers.
    pub headers: Option<OwnedHeaders>,
    /// Required delivery opaque (defaults to `()` if not required).
    pub delivery_opaque: (),
}

impl<'a, K: ToBytes + ?Sized, P: ToBytes + ?Sized> BaseRecord<'a, K, P> {
    /// Sets the destination partition of the record.
    pub fn partition(mut self, partition: i32) -> BaseRecord<'a, K, P> {
        self.partition = Some(partition);
        self
    }

    /// Sets the payload of the record.
    pub fn payload(mut self, payload: &'a P) -> BaseRecord<'a, K, P> {
        self.payload = Some(payload);
        self
    }

    /// Sets the key of the record.
    pub fn key(mut self, key: &'a K) -> BaseRecord<'a, K, P> {
        self.key = Some(key);
        self
    }

    /// Sets the timestamp of the record.
    ///
    /// Note that Kafka represents timestamps as the number of milliseconds
    /// since the Unix epoch.
    pub fn timestamp(mut self, timestamp: i64) -> BaseRecord<'a, K, P> {
        self.timestamp = Some(timestamp);
        self
    }

    /// Sets the headers of the record.
    pub fn headers(mut self, headers: OwnedHeaders) -> BaseRecord<'a, K, P> {
        self.headers = Some(headers);
        self
    }

    /// Creates a new record with the specified topic name.
    pub fn to(topic: &'a str) -> BaseRecord<'a, K, P> {
        BaseRecord {
            topic,
            partition: None,
            payload: None,
            key: None,
            timestamp: None,
            headers: None,
            delivery_opaque: (),
        }
    }
}

/// Common trait for all producers.
pub trait Producer<C = DefaultProducerContext>
where
    C: ProducerContext,
{
}

/// Producer-specific context.
pub trait ProducerContext: ClientContext {}

/// An inert producer context that can be used when customizations are not
/// required.
#[derive(Clone)]
pub struct DefaultProducerContext;

impl ClientContext for DefaultProducerContext {}
impl ProducerContext for DefaultProducerContext {}

#[async_trait::async_trait]
impl FromClientConfig for BaseProducer {
    async fn from_config(config: &ClientConfig) -> KafkaResult<BaseProducer> {
        BaseProducer::from_config_and_context(config, DefaultProducerContext).await
    }
}

#[async_trait::async_trait]
impl<C> FromClientConfigAndContext<C> for BaseProducer<C>
where
    C: ProducerContext,
{
    async fn from_config_and_context(
        config: &ClientConfig,
        _context: C,
    ) -> KafkaResult<BaseProducer<C>> {
        let config_json = serde_json::to_string(&config.conf_map)
            .map_err(|e| KafkaError::ClientCreation(e.to_string()))?;
        let config: ProducerConfig = serde_json::from_str(&config_json)
            .map_err(|e| KafkaError::ClientCreation(e.to_string()))?;
        let addr = config
            .bootstrap_servers
            .parse::<SocketAddr>()
            .map_err(|e| KafkaError::ClientCreation(e.to_string()))?;
        let p = BaseProducer {
            _context,
            _config: config,
            ep: Endpoint::bind("0.0.0.0:0")
                .await
                .map_err(|e| KafkaError::ClientCreation(e.to_string()))?,
            addr,
            buffer: Mutex::new(vec![]),
        };
        Ok(p)
    }
}

/// Lowest level Kafka producer.
pub struct BaseProducer<C = DefaultProducerContext>
where
    C: ProducerContext,
{
    _context: C,
    _config: ProducerConfig,
    ep: Endpoint,
    addr: SocketAddr,
    buffer: Mutex<Vec<OwnedRecord>>,
}

/// A low-level Kafka producer with a separate thread for event handling.
pub type ThreadedProducer<C> = BaseProducer<C>;

impl<C> BaseProducer<C>
where
    C: ProducerContext,
{
    /// Sends a message to Kafka.
    pub fn send<'a, K, P>(
        &self,
        record: BaseRecord<'a, K, P>,
    ) -> Result<(), (KafkaError, BaseRecord<'a, K, P>)>
    where
        K: ToBytes + ?Sized,
        P: ToBytes + ?Sized,
    {
        // TODO: simulate queue full
        self.buffer.lock().push(record.to_owned());
        Ok(())
    }

    /// Polls the producer, returning the number of events served.
    pub async fn poll(&self) -> i32 {
        match self.poll_internal().await {
            Ok(num) => num as _,
            Err(e) => {
                error!("poll producer error: {}", e);
                0
            }
        }
    }

    /// Polls the producer, returning the number of events served.
    async fn poll_internal(&self) -> KafkaResult<u32> {
        let records = std::mem::take(&mut *self.buffer.lock());
        if records.is_empty() {
            return Ok(0);
        }
        let num = records.len() as u32;
        let req = Request::Produce { records };
        let (tx, mut rx) = self.ep.connect1(self.addr).await?;
        tx.send(Box::new(req)).await?;
        let res = *rx.recv().await?.downcast::<KafkaResult<()>>().unwrap();
        res?;
        Ok(num)
    }
}

impl<C> BaseProducer<C>
where
    C: ProducerContext,
{
    /// Flushes any pending messages.
    pub async fn flush<T: Into<Timeout>>(&self, timeout: T) {
        todo!()
    }

    /// Enable sending transactions with this producer.
    pub async fn init_transactions<T: Into<Timeout>>(&self, timeout: T) -> KafkaResult<()> {
        todo!()
    }

    /// Begins a new transaction.
    pub fn begin_transaction(&self) -> KafkaResult<()> {
        todo!()
    }

    /// Commits the current transaction.
    pub async fn commit_transaction<T: Into<Timeout>>(&self, timeout: T) -> KafkaResult<()> {
        todo!()
    }

    /// Aborts the current transaction.
    pub async fn abort_transaction<T: Into<Timeout>>(&self, timeout: T) -> KafkaResult<()> {
        todo!()
    }
}

/// Producer configs.
///
/// <https://kafka.apache.org/documentation/#producerconfigs>
#[derive(Debug, Default, Deserialize)]
struct ProducerConfig {
    #[serde(rename = "bootstrap.servers")]
    bootstrap_servers: String,

    #[serde(rename = "transactional.id")]
    transactional_id: Option<String>,

    /// Local message timeout.
    #[serde(
        rename = "message.timeout.ms",
        deserialize_with = "super::from_str",
        default = "default_message_timeout_ms"
    )]
    message_timeout_ms: u32,
}

const fn default_message_timeout_ms() -> u32 {
    300_000
}
