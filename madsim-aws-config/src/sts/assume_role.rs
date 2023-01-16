use aws_types::credentials::{future, ProvideCredentials, SharedCredentialsProvider};
use aws_types::region::Region;
use std::time::Duration;

use crate::meta::credentials::LazyCachingCredentialsProvider;
use crate::provider_config::ProviderConfig;

#[derive(Debug)]
pub struct AssumeRoleProvider {
    cache: LazyCachingCredentialsProvider,
}

impl AssumeRoleProvider {
    pub fn builder(_role: impl Into<String>) -> AssumeRoleProviderBuilder {
        AssumeRoleProviderBuilder::new("default")
    }
}

#[derive(Debug)]
pub struct AssumeRoleProviderBuilder {}

impl AssumeRoleProviderBuilder {
    pub fn new(_role: impl Into<String>) -> Self {
        Self {}
    }

    pub fn external_id(mut self, _id: impl Into<String>) -> Self {
        self
    }

    pub fn session_name(mut self, _name: impl Into<String>) -> Self {
        self
    }

    pub fn session_length(mut self, _length: Duration) -> Self {
        self
    }

    pub fn region(mut self, _region: Region) -> Self {
        self
    }

    pub fn connection(mut self, _conn: impl aws_smithy_client::bounds::SmithyConnector) -> Self {
        self
    }

    pub fn configure(mut self, _conf: &ProviderConfig) -> Self {
        self
    }

    pub fn build(self, _provider: impl Into<SharedCredentialsProvider>) -> AssumeRoleProvider {
        AssumeRoleProvider {
            cache: LazyCachingCredentialsProvider::builder().build(),
        }
    }
}

impl ProvideCredentials for AssumeRoleProvider {
    fn provide_credentials<'a>(&'a self) -> future::ProvideCredentials<'a>
    where
        Self: 'a,
    {
        self.cache.provide_credentials()
    }
}
