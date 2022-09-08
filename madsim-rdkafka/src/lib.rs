#![allow(unused_variables)]

pub mod client;
pub mod config;
pub mod consumer;
pub mod error;
pub mod message;
pub mod metadata;
pub mod producer;
pub mod topic_partition_list;
pub mod types;
pub mod util;

pub use crate::client::ClientContext;
pub use crate::config::ClientConfig;
pub use crate::message::Message;
pub use crate::topic_partition_list::{Offset, TopicPartitionList};
