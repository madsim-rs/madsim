use crate::Statistics;

use tracing::info;

/// Client-level context.
pub trait ClientContext: Send + Sync + 'static {
    /// Receives the decoded statistics of the librdkafka client. To enable, the
    /// `statistics.interval.ms` configuration parameter must be specified.
    ///
    /// The default implementation logs the statistics at the `info` log level.
    fn stats(&self, statistics: Statistics) {
        info!("Client stats: {:?}", statistics);
    }

    fn rewrite_broker_addr(&self, addr: BrokerAddr) -> BrokerAddr {
        addr
    }
}

/// An empty [`ClientContext`] that can be used when no customizations are needed.
#[derive(Clone, Debug, Default)]
pub struct DefaultClientContext;

impl ClientContext for DefaultClientContext {}

/// Describes the address of a broker in a Kafka cluster.
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct BrokerAddr {
    /// The host name.
    pub host: String,
    /// The port, either as a decimal number or the name of a service in
    /// the services database.
    pub port: String,
}
