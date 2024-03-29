/// Metadata container.
pub struct Metadata {
    pub(crate) topics: Vec<MetadataTopic>,
}

impl Metadata {
    /// Returns the metadata information for all the topics in the cluster.
    pub fn topics(&self) -> &[MetadataTopic] {
        &self.topics
    }
}

/// Topic metadata information.
pub struct MetadataTopic {
    pub(crate) name: String,
    pub(crate) partitions: Vec<MetadataPartition>,
}

impl MetadataTopic {
    /// Returns the name of the topic.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the partition metadata information for all the partitions.
    pub fn partitions(&self) -> &[MetadataPartition] {
        &self.partitions
    }
}

/// Partition metadata information.
pub struct MetadataPartition {
    pub(crate) id: i32,
}

impl MetadataPartition {
    /// Returns the id of the partition.
    pub fn id(&self) -> i32 {
        self.id
    }
}
