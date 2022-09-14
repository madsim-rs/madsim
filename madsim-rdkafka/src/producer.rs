use std::marker::PhantomData;

use crate::{
    client::ClientContext,
    config::FromClientConfigAndContext,
    error::{KafkaError, KafkaResult},
    message::{OwnedHeaders, ToBytes},
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
    /// Flushes any pending messages.
    fn flush<T: Into<Timeout>>(&self, timeout: T);

    /// Enable sending transactions with this producer.
    fn init_transactions<T: Into<Timeout>>(&self, timeout: T) -> KafkaResult<()>;

    /// Begins a new transaction.
    fn begin_transaction(&self) -> KafkaResult<()>;

    /// Commits the current transaction.
    fn commit_transaction<T: Into<Timeout>>(&self, timeout: T) -> KafkaResult<()>;

    /// Aborts the current transaction.
    fn abort_transaction<T: Into<Timeout>>(&self, timeout: T) -> KafkaResult<()>;
}

/// Producer-specific context.
pub trait ProducerContext: ClientContext {}

/// An inert producer context that can be used when customizations are not
/// required.
#[derive(Clone)]
pub struct DefaultProducerContext;

impl ClientContext for DefaultProducerContext {}
impl ProducerContext for DefaultProducerContext {}

impl<C> FromClientConfigAndContext<C> for BaseProducer<C>
where
    C: ProducerContext,
{
    /// Creates a new `BaseProducer` starting from a configuration and a
    /// context.
    fn from_config_and_context(config: &ClientConfig, context: C) -> KafkaResult<BaseProducer<C>> {
        todo!()
    }
}

/// Lowest level Kafka producer.
pub struct BaseProducer<C = DefaultProducerContext>
where
    C: ProducerContext,
{
    _runtime: PhantomData<C>,
}

/// A low-level Kafka producer with a separate thread for event handling.
pub type ThreadedProducer<C> = BaseProducer<C>;

impl<C> BaseProducer<C>
where
    C: ProducerContext,
{
    pub fn send<'a, K, P>(
        &self,
        record: BaseRecord<'a, K, P>,
    ) -> Result<(), (KafkaError, BaseRecord<'a, K, P>)>
    where
        K: ToBytes + ?Sized,
        P: ToBytes + ?Sized,
    {
        todo!()
    }
}

impl<C> Producer<C> for BaseProducer<C>
where
    C: ProducerContext,
{
    fn flush<T: Into<Timeout>>(&self, timeout: T) {
        todo!()
    }

    fn init_transactions<T: Into<Timeout>>(&self, timeout: T) -> KafkaResult<()> {
        todo!()
    }

    fn begin_transaction(&self) -> KafkaResult<()> {
        todo!()
    }

    fn commit_transaction<T: Into<Timeout>>(&self, timeout: T) -> KafkaResult<()> {
        todo!()
    }

    fn abort_transaction<T: Into<Timeout>>(&self, timeout: T) -> KafkaResult<()> {
        todo!()
    }
}

#[derive(Debug, Default)]
struct ProducerConfig {
    bootstrap_servers: String,
    transactional_id: Option<String>,
}

impl ProducerConfig {
    fn from_kv(kv: impl IntoIterator<Item = (String, String)>) -> Self {
        let mut cfg = Self::default();
        for (k, v) in kv {
            match k.as_str() {
                "bootstrap.servers" => cfg.bootstrap_servers = v,
                "transactional.id" => cfg.transactional_id = Some(v.clone()),
                _ => panic!("invalid key: {}", k),
            }
        }
        cfg
    }
}
