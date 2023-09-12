use super::{BaseRecord, DeliveryResult, IntoOpaque, ProducerContext, ThreadedProducer};
use crate::{
    client::{BrokerAddr, DefaultClientContext},
    config::{FromClientConfig, FromClientConfigAndContext},
    error::{KafkaError, KafkaResult},
    message::{Message, OwnedHeaders, OwnedMessage, ToBytes},
    util::Timeout,
    ClientConfig, ClientContext,
};
use futures_channel::oneshot;
use futures_util::FutureExt;

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

/// A record for the future producer.
///
/// Like [`BaseRecord`], but specific to the [`FutureProducer`]. The only
/// difference is that the [FutureRecord] doesn't provide custom delivery opaque
/// object.
#[derive(Debug)]
pub struct FutureRecord<'a, K: ToBytes + ?Sized, P: ToBytes + ?Sized> {
    /// Required destination topic.
    pub topic: &'a str,
    /// Optional destination partition.
    pub partition: Option<i32>,
    /// Optional payload.
    pub payload: Option<&'a P>,
    /// Optional key.
    pub key: Option<&'a K>,
    /// Optional timestamp.
    pub timestamp: Option<i64>,
    /// Optional message headers.
    pub headers: Option<OwnedHeaders>,
}

impl<'a, K: ToBytes + ?Sized, P: ToBytes + ?Sized> FutureRecord<'a, K, P> {
    /// Creates a new record with the specified topic name.
    pub fn to(topic: &'a str) -> FutureRecord<'a, K, P> {
        FutureRecord {
            topic,
            partition: None,
            payload: None,
            key: None,
            timestamp: None,
            headers: None,
        }
    }

    fn from_base_record<D: IntoOpaque>(
        base_record: BaseRecord<'a, K, P, D>,
    ) -> FutureRecord<'a, K, P> {
        FutureRecord {
            topic: base_record.topic,
            partition: base_record.partition,
            key: base_record.key,
            payload: base_record.payload,
            timestamp: base_record.timestamp,
            headers: base_record.headers,
        }
    }

    /// Sets the destination partition of the record.
    pub fn partition(mut self, partition: i32) -> FutureRecord<'a, K, P> {
        self.partition = Some(partition);
        self
    }

    /// Sets the destination payload of the record.
    pub fn payload(mut self, payload: &'a P) -> FutureRecord<'a, K, P> {
        self.payload = Some(payload);
        self
    }

    /// Sets the destination key of the record.
    pub fn key(mut self, key: &'a K) -> FutureRecord<'a, K, P> {
        self.key = Some(key);
        self
    }

    /// Sets the destination timestamp of the record.
    pub fn timestamp(mut self, timestamp: i64) -> FutureRecord<'a, K, P> {
        self.timestamp = Some(timestamp);
        self
    }

    /// Sets the headers of the record.
    pub fn headers(mut self, headers: OwnedHeaders) -> FutureRecord<'a, K, P> {
        self.headers = Some(headers);
        self
    }

    fn into_base_record<D: IntoOpaque>(self, delivery_opaque: D) -> BaseRecord<'a, K, P, D> {
        BaseRecord {
            topic: self.topic,
            partition: self.partition,
            key: self.key,
            payload: self.payload,
            timestamp: self.timestamp,
            headers: self.headers,
            delivery_opaque,
        }
    }
}

/// The [`ProducerContext`] used by the [`FutureProducer`].
///
/// This context will use a [`Future`] as its `DeliveryOpaque` and will complete
/// the future when the message is delivered (or failed to).
#[derive(Clone)]
pub struct FutureProducerContext<C: ClientContext + 'static> {
    wrapped_context: C,
}

/// Represents the result of message production as performed from the
/// `FutureProducer`.
///
/// If message delivery was successful, `OwnedDeliveryResult` will return the
/// partition and offset of the message. If the message failed to be delivered
/// an error will be returned, together with an owned copy of the original
/// message.
pub type OwnedDeliveryResult = Result<(i32, i64), (KafkaError, OwnedMessage)>;

// Delegates all the methods calls to the wrapped context.
impl<C: ClientContext + 'static> ClientContext for FutureProducerContext<C> {
    fn rewrite_broker_addr(&self, addr: BrokerAddr) -> BrokerAddr {
        self.wrapped_context.rewrite_broker_addr(addr)
    }
}

impl<C: ClientContext + 'static> ProducerContext for FutureProducerContext<C> {
    type DeliveryOpaque = Box<oneshot::Sender<OwnedDeliveryResult>>;

    fn delivery(
        &self,
        delivery_result: &DeliveryResult<'_>,
        tx: Box<oneshot::Sender<OwnedDeliveryResult>>,
    ) {
        let owned_delivery_result = match *delivery_result {
            Ok(ref message) => Ok((message.partition(), message.offset())),
            Err((ref error, ref message)) => Err((error.clone(), message.detach())),
        };
        let _ = tx.send(owned_delivery_result); // TODO: handle error
    }
}

/// A producer that returns a [`Future`] for every message being produced.
///
/// Since message production in rdkafka is asynchronous, the caller cannot
/// immediately know if the delivery of the message was successful or not. The
/// FutureProducer provides this information in a [`Future`], which will be
/// completed once the information becomes available.
///
/// This producer has an internal polling thread and as such it doesn't need to
/// be polled. It can be cheaply cloned to get a reference to the same
/// underlying producer. The internal polling thread will be terminated when the
/// `FutureProducer` goes out of scope.
#[must_use = "Producer polling thread will stop immediately if unused"]
// NOTE: <R = DefaultRuntime> is ignored for now
pub struct FutureProducer<C = DefaultClientContext>
where
    C: ClientContext + 'static,
{
    producer: Arc<ThreadedProducer<FutureProducerContext<C>>>,
}

impl<C> Clone for FutureProducer<C>
where
    C: ClientContext + 'static,
{
    fn clone(&self) -> FutureProducer<C> {
        FutureProducer {
            producer: self.producer.clone(),
        }
    }
}

#[async_trait::async_trait]
impl FromClientConfig for FutureProducer<DefaultClientContext> {
    async fn from_config(
        config: &ClientConfig,
    ) -> KafkaResult<FutureProducer<DefaultClientContext>> {
        FutureProducer::from_config_and_context(config, DefaultClientContext).await
    }
}

#[async_trait::async_trait]
impl<C> FromClientConfigAndContext<C> for FutureProducer<C>
where
    C: ClientContext + 'static,
{
    async fn from_config_and_context(
        config: &ClientConfig,
        context: C,
    ) -> KafkaResult<FutureProducer<C>> {
        let future_context = FutureProducerContext {
            wrapped_context: context,
        };
        let threaded_producer =
            ThreadedProducer::from_config_and_context(config, future_context).await?;
        Ok(FutureProducer {
            producer: Arc::new(threaded_producer),
        })
    }
}

/// A [`Future`] wrapping the result of the message production.
///
/// Once completed, the future will contain an `OwnedDeliveryResult` with
/// information on the delivery status of the message. If the producer is
/// dropped before the delivery status is received, the future will instead
/// resolve with [`oneshot::Canceled`].
pub struct DeliveryFuture {
    rx: oneshot::Receiver<OwnedDeliveryResult>,
}

impl Future for DeliveryFuture {
    // the original crate returns `futures_channel::oneshot::Canceled`.
    type Output = Result<OwnedDeliveryResult, oneshot::Canceled>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.rx.poll_unpin(cx)
    }
}

impl<C> FutureProducer<C>
where
    C: ClientContext + 'static,
{
    /// Sends a message to Kafka, returning the result of the send.
    ///
    /// The `queue_timeout` parameter controls how long to retry for if the
    /// librdkafka producer queue is full. Set it to `Timeout::Never` to retry
    /// forever or `Timeout::After(0)` to never block. If the timeout is reached
    /// and the queue is still full, an [`RDKafkaErrorCode::QueueFull`] error will
    /// be reported in the [`OwnedDeliveryResult`].
    ///
    /// Keep in mind that `queue_timeout` only applies to the first phase of the
    /// send operation. Once the message is queued, the underlying librdkafka
    /// client has separate timeout parameters that apply, like
    /// `delivery.timeout.ms`.
    ///
    /// See also the [`FutureProducer::send_result`] method, which will not
    /// retry the queue operation if the queue is full.
    pub async fn send<K, P, T>(
        &self,
        record: FutureRecord<'_, K, P>,
        _queue_timeout: T,
    ) -> OwnedDeliveryResult
    where
        K: ToBytes + ?Sized,
        P: ToBytes + ?Sized,
        T: Into<Timeout>,
    {
        self.send_result(record)
            .map_err(|_| todo!("handle queue full"))
            .unwrap()
            .await
            .expect("producer is dropped before the delivery status is received")
    }

    /// Like [`FutureProducer::send`], but if enqueuing fails, an error will be
    /// returned immediately, alongside the [`FutureRecord`] provided.
    pub fn send_result<'a, K, P>(
        &self,
        record: FutureRecord<'a, K, P>,
    ) -> Result<DeliveryFuture, (KafkaError, FutureRecord<'a, K, P>)>
    where
        K: ToBytes + ?Sized,
        P: ToBytes + ?Sized,
    {
        let (tx, rx) = oneshot::channel();
        let base_record = record.into_base_record(Box::new(tx));
        self.producer
            .send(base_record)
            .map(|()| DeliveryFuture { rx })
            .map_err(|(e, record)| (e, FutureRecord::from_base_record(record)))
    }
}
