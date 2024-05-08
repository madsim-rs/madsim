use crate::config::RDKafkaLogLevel;
use crate::error::KafkaError;
use crate::Statistics;
use std::error::Error;
use tracing::*;

/// Client-level context.
pub trait ClientContext: Send + Sync + 'static {
    fn enable_refresh_oauth_token(&self) -> bool {
        false
    }

    fn log(&self, level: RDKafkaLogLevel, fac: &str, log_message: &str) {
        match level {
            RDKafkaLogLevel::Emerg
            | RDKafkaLogLevel::Alert
            | RDKafkaLogLevel::Critical
            | RDKafkaLogLevel::Error => {
                error!(target: "librdkafka", "librdkafka: {} {}", fac, log_message)
            }
            RDKafkaLogLevel::Warning => {
                warn!(target: "librdkafka", "librdkafka: {} {}", fac, log_message)
            }
            RDKafkaLogLevel::Notice => {
                info!(target: "librdkafka", "librdkafka: {} {}", fac, log_message)
            }
            RDKafkaLogLevel::Info => {
                info!(target: "librdkafka", "librdkafka: {} {}", fac, log_message)
            }
            RDKafkaLogLevel::Debug => {
                debug!(target: "librdkafka", "librdkafka: {} {}", fac, log_message)
            }
        }
    }

    /// Receives the decoded statistics of the librdkafka client. To enable, the
    /// `statistics.interval.ms` configuration parameter must be specified.
    ///
    /// The default implementation logs the statistics at the `info` log level.
    fn stats(&self, statistics: Statistics) {
        info!("Client stats: {:?}", statistics);
    }

    fn stats_raw(&self, statistics: &[u8]) {
        match serde_json::from_slice(statistics) {
            Ok(stats) => self.stats(stats),
            Err(e) => error!("Could not parse statistics JSON: {}", e),
        }
    }

    fn error(&self, error: KafkaError, reason: &str) {
        error!("librdkafka: {}: {}", error, reason);
    }

    fn rewrite_broker_addr(&self, addr: BrokerAddr) -> BrokerAddr {
        addr
    }

    fn generate_oauth_token(
        &self,
        _oauthbearer_config: Option<&str>,
    ) -> Result<OAuthToken, Box<dyn Error>> {
        Err("Default implementation of generate_oauth_token must be overridden".into())
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

/// A generated OAuth token and its associated metadata.
///
/// When using the `OAUTHBEARER` SASL authentication method, this type is
/// returned from [`ClientContext::generate_oauth_token`]. The token and
/// principal name must not contain embedded null characters.
///
/// Specifying SASL extensions is not currently supported.
pub struct OAuthToken {
    /// The token value to set.
    pub token: String,
    /// The Kafka principal name associated with the token.
    pub principal_name: String,
    /// When the token expires, in number of milliseconds since the Unix epoch.
    pub lifetime_ms: i64,
}
