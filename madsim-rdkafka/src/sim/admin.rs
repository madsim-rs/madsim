// MIT License
//
// Copyright (c) 2016 Federico Giraud
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

use std::future::Future;
use std::net::SocketAddr;

use madsim::net::Endpoint;
use serde::Deserialize;

use crate::{
    client::{ClientContext, DefaultClientContext},
    config::{FromClientConfig, FromClientConfigAndContext},
    error::{KafkaError, KafkaResult},
    sim_broker::Request,
    types::RDKafkaErrorCode,
    util::Timeout,
    ClientConfig,
};

pub struct AdminClient<C: ClientContext> {
    _context: C,
    _config: AdminClientConfig,
    ep: Endpoint,
    addr: SocketAddr,
}

#[async_trait::async_trait]
impl FromClientConfig for AdminClient<DefaultClientContext> {
    async fn from_config(config: &ClientConfig) -> KafkaResult<Self> {
        AdminClient::from_config_and_context(config, DefaultClientContext).await
    }
}

#[async_trait::async_trait]
impl<C: ClientContext> FromClientConfigAndContext<C> for AdminClient<C> {
    async fn from_config_and_context(
        config: &ClientConfig,
        _context: C,
    ) -> KafkaResult<AdminClient<C>> {
        let config_json = serde_json::to_string(&config.conf_map)
            .map_err(|e| KafkaError::ClientCreation(e.to_string()))?;
        let config: AdminClientConfig = serde_json::from_str(&config_json)
            .map_err(|e| KafkaError::ClientCreation(e.to_string()))?;
        let addr: SocketAddr = madsim::net::lookup_host(&config.bootstrap_servers)
            .await
            .map_err(|e| KafkaError::ClientCreation(e.to_string()))?
            .next()
            .ok_or_else(|| KafkaError::ClientCreation("invalid host or ip".into()))?;
        Ok(AdminClient {
            _context,
            _config: config,
            ep: Endpoint::bind("0.0.0.0:0").await?,
            addr,
        })
    }
}

impl<C> AdminClient<C>
where
    C: ClientContext,
{
    /// Creates new topics according to the provided [`NewTopic`] specifications.
    pub async fn create_topics<'a>(
        &self,
        topics: impl IntoIterator<Item = &'a NewTopic<'a>>,
        _opts: &AdminOptions,
    ) -> KafkaResult<Vec<TopicResult>> {
        let mut results = vec![];
        for topic in topics {
            let req = Request::CreateTopic {
                name: topic.name.to_string(),
                partitions: topic.num_partitions as usize,
            };
            let (tx, mut rx) = self.ep.connect1(self.addr).await?;
            tx.send(Box::new(req)).await?;
            let res = match *rx.recv().await?.downcast::<KafkaResult<()>>().unwrap() {
                Ok(()) => Ok(topic.name.to_string()),
                Err(e) => todo!("failed to create topic: {}", e),
            };
            results.push(res);
        }
        Ok(results)
    }

    /// Deletes the named groups.
    pub fn delete_groups(
        &self,
        _group_names: &[&str],
        _opts: &AdminOptions,
    ) -> impl Future<Output = KafkaResult<Vec<GroupResult>>> {
        // no-op
        std::future::ready(Ok(vec![]))
    }
}

/// Options for an admin API request.
#[derive(Default)]
pub struct AdminOptions {
    request_timeout: Option<Timeout>,
    operation_timeout: Option<Timeout>,
    validate_only: bool,
    broker_id: Option<i32>,
}

impl AdminOptions {
    /// Creates a new `AdminOptions`.
    pub fn new() -> AdminOptions {
        AdminOptions::default()
    }

    /// Sets the overall request timeout, including broker lookup, request
    /// transmission, operation time on broker, and response.
    pub fn request_timeout<T: Into<Timeout>>(mut self, timeout: Option<T>) -> Self {
        self.request_timeout = timeout.map(Into::into);
        self
    }

    /// Sets the broker's operation timeout, such as the timeout for
    /// CreateTopics to complete the creation of topics on the controller before
    /// returning a result to the application.
    pub fn operation_timeout<T: Into<Timeout>>(mut self, timeout: Option<T>) -> Self {
        self.operation_timeout = timeout.map(Into::into);
        self
    }

    /// Tells the broker to only validate the request, without performing the
    /// requested operation.
    pub fn validate_only(mut self, validate_only: bool) -> Self {
        self.validate_only = validate_only;
        self
    }

    /// Override what broker the admin request will be sent to.
    pub fn broker_id<T: Into<Option<i32>>>(mut self, broker_id: T) -> Self {
        self.broker_id = broker_id.into();
        self
    }
}

/// Configuration for a CreateTopic operation.
#[derive(Debug)]
pub struct NewTopic<'a> {
    /// The name of the new topic.
    pub name: &'a str,
    /// The initial number of partitions.
    pub num_partitions: i32,
    /// The initial replication configuration.
    pub replication: TopicReplication<'a>,
    /// The initial configuration parameters for the topic.
    pub config: Vec<(&'a str, &'a str)>,
}

impl<'a> NewTopic<'a> {
    /// Creates a new `NewTopic`.
    pub fn new(
        name: &'a str,
        num_partitions: i32,
        replication: TopicReplication<'a>,
    ) -> NewTopic<'a> {
        NewTopic {
            name,
            num_partitions,
            replication,
            config: Vec::new(),
        }
    }

    /// Sets a new parameter in the initial topic configuration.
    pub fn set(mut self, key: &'a str, value: &'a str) -> NewTopic<'a> {
        self.config.push((key, value));
        self
    }
}

/// Configuration for a CreatePartitions operation.
pub struct NewPartitions<'a> {
    /// The name of the topic to which partitions should be added.
    pub topic_name: &'a str,
    /// The total number of partitions after the operation completes.
    pub new_partition_count: usize,
    /// The replica assignments for the new partitions.
    pub assignment: Option<PartitionAssignment<'a>>,
}

impl<'a> NewPartitions<'a> {
    /// Creates a new `NewPartitions`.
    pub fn new(topic_name: &'a str, new_partition_count: usize) -> NewPartitions<'a> {
        NewPartitions {
            topic_name,
            new_partition_count,
            assignment: None,
        }
    }

    /// Sets the partition replica assignment for the new partitions. Only
    /// assignments for newly created replicas should be included.
    pub fn assign(mut self, assignment: PartitionAssignment<'a>) -> NewPartitions<'a> {
        self.assignment = Some(assignment);
        self
    }
}

/// An assignment of partitions to replicas.
///
/// Each element in the outer slice corresponds to the partition with that
/// index. The inner slice specifies the broker IDs to which replicas of that
/// partition should be assigned.
pub type PartitionAssignment<'a> = &'a [&'a [i32]];

/// Replication configuration for a new topic.
#[derive(Debug)]
pub enum TopicReplication<'a> {
    /// All partitions should use the same fixed replication factor.
    Fixed(i32),
    /// Each partition should use the replica assignment from
    /// `PartitionAssignment`.
    Variable(PartitionAssignment<'a>),
}

/// The result of an individual CreateTopic, DeleteTopic, or
/// CreatePartition operation.
pub type TopicResult = Result<String, (String, RDKafkaErrorCode)>;

/// The result of a DeleteGroup operation.
pub type GroupResult = Result<String, (String, RDKafkaErrorCode)>;

/// AdminClient configs.
///
/// <https://kafka.apache.org/documentation/#adminclientconfigs>
#[derive(Debug, Default, Deserialize)]
struct AdminClientConfig {
    #[serde(rename = "bootstrap.servers")]
    bootstrap_servers: String,
}
