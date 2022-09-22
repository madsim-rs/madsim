pub mod admin;
pub(crate) mod broker;
pub mod client;
pub mod config;
pub mod consumer;
pub mod error;
pub mod message;
pub mod metadata;
pub mod producer;
pub(crate) mod sim_broker;
pub mod topic_partition_list;
pub mod types;
pub mod util;

pub use self::client::ClientContext;
pub use self::config::ClientConfig;
pub use self::message::{Message, Timestamp};
pub use self::sim_broker::SimBroker;
pub use self::topic_partition_list::{Offset, TopicPartitionList};

// custom deserialize function for serde
fn from_str<'de, D, T>(de: D) -> Result<T, D::Error>
where
    D: serde::Deserializer<'de>,
    T: std::str::FromStr,
    T::Err: std::fmt::Display,
{
    let s: &str = serde::Deserialize::deserialize(de)?;
    s.parse().map_err(serde::de::Error::custom)
}
