use std::{net::SocketAddr, ops::Deref, sync::Arc, time::Duration};

use madsim::net::Endpoint;
use serde::Deserialize;
use spin::Mutex;
use tracing::*;

use crate::{
    broker::OwnedRecord,
    client::ClientContext,
    config::{FromClientConfig, FromClientConfigAndContext},
    error::{KafkaError, KafkaResult, RDKafkaError, RDKafkaErrorCode},
    message::{OwnedHeaders, ToBytes},
    sim_broker::Request,
    util::{IntoOpaque, Timeout},
    ClientConfig,
};

pub use crate::message::DeliveryResult;

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
pub trait ProducerContext: ClientContext {
    type DeliveryOpaque: IntoOpaque;
    fn delivery(&self, delivery_result: &DeliveryResult<'_>, delivery_opaque: Self::DeliveryOpaque);
}

/// An inert producer context that can be used when customizations are not
/// required.
#[derive(Clone)]
pub struct DefaultProducerContext;

impl ClientContext for DefaultProducerContext {}
impl ProducerContext for DefaultProducerContext {
    type DeliveryOpaque = ();
    fn delivery(&self, _: &DeliveryResult<'_>, _: Self::DeliveryOpaque) {}
}

#[async_trait::async_trait]
impl FromClientConfig for BaseProducer {
    async fn from_config(config: &ClientConfig) -> KafkaResult<Self> {
        BaseProducer::from_config_and_context(config, DefaultProducerContext).await
    }
}

#[async_trait::async_trait]
impl<C> FromClientConfigAndContext<C> for BaseProducer<C>
where
    C: ProducerContext,
{
    async fn from_config_and_context(config: &ClientConfig, _context: C) -> KafkaResult<Self> {
        let config_json = serde_json::to_string(&config.conf_map)
            .map_err(|e| KafkaError::ClientCreation(e.to_string()))?;
        let config: ProducerConfig = serde_json::from_str(&config_json)
            .map_err(|e| KafkaError::ClientCreation(e.to_string()))?;
        let addr: SocketAddr = madsim::net::lookup_host(&config.bootstrap_servers)
            .await
            .map_err(|e| KafkaError::ClientCreation(e.to_string()))?
            .next()
            .ok_or_else(|| KafkaError::ClientCreation("invalid host or ip".into()))?;
        let p = BaseProducer {
            _context,
            config,
            ep: Endpoint::bind("0.0.0.0:0")
                .await
                .map_err(|e| KafkaError::ClientCreation(e.to_string()))?,
            addr,
            inner: Mutex::new(Inner::default()),
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
    config: ProducerConfig,
    ep: Endpoint,
    addr: SocketAddr,
    inner: Mutex<Inner>,
}

#[derive(Debug, Default)]
enum Inner {
    #[default]
    Init,
    NonTxn {
        buffer: Vec<OwnedRecord>,
    },
    Txn {
        /// Indicate whether the producer is in a transaction.
        in_txn: bool,
        // We simulate transaction by buffering all records and sending them in a batch.
        buffer: Vec<OwnedRecord>,
    },
}

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
        let mut inner = self.inner.lock();
        if matches!(&*inner, Inner::Init) {
            *inner = Inner::NonTxn { buffer: vec![] };
        }
        match &mut *inner {
            Inner::NonTxn { buffer } => {
                if buffer.len() > 10 {
                    // simulate queue full
                    return Err((
                        KafkaError::MessageProduction(RDKafkaErrorCode::QueueFull),
                        record,
                    ));
                }
                buffer.push(record.to_owned());
            }
            Inner::Txn { in_txn, buffer } => {
                assert!(
                    *in_txn,
                    "messages should only be sent when a transaction is active"
                );
                buffer.push(record.to_owned());
            }
            Inner::Init => unreachable!(),
        }
        Ok(())
    }

    /// Polls the producer, returning the number of events served.
    pub async fn poll<T: Into<Timeout>>(&self, timeout: T) -> i32 {
        _ = self.flush(timeout).await;
        0
    }

    async fn flush_internal(&self) -> KafkaResult<()> {
        let records = match &mut *self.inner.lock() {
            Inner::NonTxn { buffer } if !buffer.is_empty() => std::mem::take(buffer),
            _ => return Ok(()),
        };
        debug!("flushing {} records", records.len());
        let req = Request::Produce { records };
        let (tx, mut rx) = self.ep.connect1(self.addr).await?;
        tx.send(Box::new(req)).await?;
        *rx.recv().await?.downcast::<KafkaResult<()>>().unwrap()
    }

    /// Flushes any pending messages.
    pub async fn flush<T: Into<Timeout>>(&self, timeout: T) -> KafkaResult<()> {
        let future = self.flush_internal();
        match timeout.into() {
            Timeout::After(dur) => madsim::time::timeout(dur, future)
                .await
                .map_err(|_| KafkaError::Flush(RDKafkaErrorCode::RequestTimedOut))?,
            Timeout::Never => future.await,
        }
    }

    /// Enable sending transactions with this producer.
    pub async fn init_transactions<T: Into<Timeout>>(&self, _timeout: T) -> KafkaResult<()> {
        debug!("init transactions");
        if self.config.transactional_id.is_none() {
            return Err(invalid_transaction_state("transactional ID not set"));
        }
        match &mut *self.inner.lock() {
            inner @ Inner::Init => {
                *inner = Inner::Txn {
                    in_txn: false,
                    buffer: vec![],
                };
                Ok(())
            }
            _ => Err(invalid_transaction_state(
                "init_transactions must be called before any operations",
            )),
        }
    }

    /// Begins a new transaction.
    pub fn begin_transaction(&self) -> KafkaResult<()> {
        debug!("begin transaction");
        match &mut *self.inner.lock() {
            Inner::Txn { in_txn, .. } if !*in_txn => *in_txn = true,
            Inner::Txn { .. } => {
                return Err(invalid_transaction_state("transaction already in progress"));
            }
            _ => return Err(invalid_transaction_state("transaction not initialized")),
        }
        Ok(())
    }

    /// Commits the current transaction.
    pub async fn commit_transaction<T: Into<Timeout>>(&self, _timeout: T) -> KafkaResult<()> {
        debug!("commit transaction");
        let records = match &mut *self.inner.lock() {
            Inner::Txn { in_txn, buffer } if *in_txn => std::mem::take(buffer),
            _ => return Err(invalid_transaction_state("no opened transaction")),
        };
        let req = Request::Produce { records };
        let (tx, mut rx) = self.ep.connect1(self.addr).await?;
        tx.send(Box::new(req)).await?;
        let res = *rx.recv().await?.downcast::<KafkaResult<()>>().unwrap();
        // TODO: simulate transaction aborted
        match &mut *self.inner.lock() {
            Inner::Txn { in_txn, .. } if *in_txn => *in_txn = false,
            _ => panic!("state changed during commit"),
        }
        res
    }

    /// Aborts the current transaction.
    pub async fn abort_transaction<T: Into<Timeout>>(&self, _timeout: T) -> KafkaResult<()> {
        debug!("abort transaction");
        match &mut *self.inner.lock() {
            Inner::Txn { in_txn, buffer } if *in_txn => {
                buffer.clear();
                *in_txn = false;
            }
            _ => return Err(invalid_transaction_state("no opened transaction")),
        }
        Ok(())
    }
}

fn invalid_transaction_state(msg: &str) -> KafkaError {
    KafkaError::Transaction(RDKafkaError::new(
        RDKafkaErrorCode::InvalidTransactionalState,
        msg,
    ))
}

/// A low-level Kafka producer with a separate thread for event handling.
pub struct ThreadedProducer<C>
where
    C: ProducerContext + 'static,
{
    base: Arc<BaseProducer<C>>,
    _task: madsim::task::FallibleTask<()>,
}

#[async_trait::async_trait]
impl FromClientConfig for ThreadedProducer<DefaultProducerContext> {
    async fn from_config(config: &ClientConfig) -> KafkaResult<Self> {
        Self::from_config_and_context(config, DefaultProducerContext).await
    }
}

#[async_trait::async_trait]
impl<C> FromClientConfigAndContext<C> for ThreadedProducer<C>
where
    C: ProducerContext + 'static,
{
    async fn from_config_and_context(config: &ClientConfig, context: C) -> KafkaResult<Self> {
        let base = Arc::new(BaseProducer::from_config_and_context(config, context).await?);
        let producer = base.clone();
        let _task = madsim::task::Builder::new()
            .name("kafka producer polling thread")
            .spawn(async move {
                loop {
                    producer.poll(None).await;
                    madsim::time::sleep(Duration::from_millis(100)).await;
                }
            })
            .cancel_on_drop();
        Ok(ThreadedProducer { base, _task })
    }
}

impl<C> Deref for ThreadedProducer<C>
where
    C: ProducerContext + 'static,
{
    type Target = BaseProducer<C>;

    fn deref(&self) -> &Self::Target {
        &self.base
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
    #[allow(dead_code)]
    message_timeout_ms: u32,
}

const fn default_message_timeout_ms() -> u32 {
    300_000
}
