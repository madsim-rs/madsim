use std::fmt::Debug;

use aws_smithy_http::endpoint::Endpoint;
use aws_types::credentials::Credentials;
use aws_types::region::Region;
use aws_types::SdkConfig;

#[derive(Debug)]
pub struct Config {
    endpoint: Endpoint,
}

impl Config {
    pub fn builder() -> Builder {
        Builder::default()
    }

    pub fn new(config: &SdkConfig) -> Self {
        Builder::from(config).build()
    }
}

#[derive(Default)]
pub struct Builder {
    region: Option<Region>,
    endpoint: Option<Endpoint>,
    credentials: Option<Credentials>,
}

impl Builder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn endpoint_resolver(mut self, endpoint_resolver: Endpoint) -> Self {
        self.endpoint = Some(endpoint_resolver);
        self
    }

    pub fn region(mut self, region: impl Into<Option<Region>>) -> Self {
        self.region = region.into();
        self
    }

    pub fn credentials_provider(mut self, credentials_provider: Credentials) -> Self {
        self.credentials = Some(credentials_provider);
        self
    }

    pub fn build(self) -> Config {
        Config {
            endpoint: self.endpoint.expect("endpoint must be set"),
        }
    }
}

impl From<&SdkConfig> for Builder {
    fn from(_input: &SdkConfig) -> Self {
        Builder::default()
    }
}

impl From<&SdkConfig> for Config {
    fn from(_sdk_config: &SdkConfig) -> Self {
        todo!("Config from SdkConfig")
    }
}
