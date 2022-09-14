#![allow(unused_variables)]

mod broker;
pub mod client;
pub mod config;
pub mod consumer;
pub mod error;
pub mod message;
pub mod metadata;
pub mod producer;
mod sim_broker;
pub mod topic_partition_list;
pub mod types;
pub mod util;

pub use crate::client::ClientContext;
pub use crate::config::ClientConfig;
pub use crate::message::Message;
pub use crate::sim_broker::SimBroker;
pub use crate::topic_partition_list::{Offset, TopicPartitionList};
