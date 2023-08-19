pub mod base_producer;
pub mod future_producer;

use serde::Deserialize;

use crate::client::ClientContext;
use crate::util::IntoOpaque;

pub use self::base_producer::*;
pub use self::future_producer::*;
pub use crate::message::DeliveryResult;

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

    #[serde(
        rename = "queue.buffering.max.messages",
        default = "default_queue_buffering_max_messages"
    )]
    queue_buffering_max_messages: usize,
}

const fn default_message_timeout_ms() -> u32 {
    300_000
}

const fn default_queue_buffering_max_messages() -> usize {
    100_000
}
