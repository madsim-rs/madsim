//! Producer and consumer configuration.

use std::collections::HashMap;

use crate::{error::KafkaResult, ClientContext};

/// The log levels supported by librdkafka.
#[derive(Copy, Clone, Debug)]
pub enum RDKafkaLogLevel {
    /// Higher priority then [`Level::Error`](log::Level::Error) from the log crate.
    Emerg = 0,
    /// Higher priority then [`Level::Error`](log::Level::Error) from the log crate.
    Alert = 1,
    /// Higher priority then [`Level::Error`](log::Level::Error) from the log crate.
    Critical = 2,
    /// Equivalent to [`Level::Error`](log::Level::Error) from the log crate.
    Error = 3,
    /// Equivalent to [`Level::Warn`](log::Level::Warn) from the log crate.
    Warning = 4,
    /// Higher priority then [`Level::Info`](log::Level::Info) from the log crate.
    Notice = 5,
    /// Equivalent to [`Level::Info`](log::Level::Info) from the log crate.
    Info = 6,
    /// Equivalent to [`Level::Debug`](log::Level::Debug) from the log crate.
    Debug = 7,
}

/// Client configuration.
#[derive(Clone, Debug)]
pub struct ClientConfig {
    pub(crate) conf_map: HashMap<String, String>,
    /// The librdkafka logging level. Refer to [`RDKafkaLogLevel`] for the list
    /// of available levels.
    pub log_level: RDKafkaLogLevel,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self::new()
    }
}

impl ClientConfig {
    /// Creates a new empty configuration.
    pub fn new() -> ClientConfig {
        ClientConfig {
            conf_map: HashMap::new(),
            log_level: RDKafkaLogLevel::Debug,
        }
    }

    /// Gets the value of a parameter in the configuration.
    pub fn get(&self, key: &str) -> Option<&str> {
        self.conf_map.get(key).map(|val| val.as_str())
    }

    /// Sets a parameter in the configuration.
    pub fn set<K, V>(&mut self, key: K, value: V) -> &mut ClientConfig
    where
        K: Into<String>,
        V: Into<String>,
    {
        self.conf_map.insert(key.into(), value.into());
        self
    }

    /// Sets the log level of the client. If not specified, the log level will be calculated based
    /// on the global log level of the log crate.
    pub fn set_log_level(&mut self, log_level: RDKafkaLogLevel) -> &mut ClientConfig {
        self.log_level = log_level;
        self
    }

    /// Uses the current configuration to create a new Consumer or Producer.
    pub async fn create<T: FromClientConfig>(&self) -> KafkaResult<T> {
        T::from_config(self).await
    }

    /// Uses the current configuration and the provided context to create a new Consumer or Producer.
    pub async fn create_with_context<C, T>(&self, context: C) -> KafkaResult<T>
    where
        C: ClientContext,
        T: FromClientConfigAndContext<C>,
    {
        T::from_config_and_context(self, context).await
    }
}

/// Create a new client based on the provided configuration.
#[async_trait::async_trait]
pub trait FromClientConfig: Sized {
    /// Creates a client from a client configuration. The default client context
    /// will be used.
    async fn from_config(_: &ClientConfig) -> KafkaResult<Self>;
}

/// Create a new client based on the provided configuration and context.
#[async_trait::async_trait]
pub trait FromClientConfigAndContext<C: ClientContext>: Sized {
    /// Creates a client from a client configuration and a client context.
    async fn from_config_and_context(_: &ClientConfig, _: C) -> KafkaResult<Self>;
}
