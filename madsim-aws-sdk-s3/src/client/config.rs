use std::fmt::Debug;

use aws_types::SdkConfig;

#[derive(Debug)]
pub struct Config {}

impl Config {
    pub fn builder() -> Builder {
        Builder::default()
    }

    pub fn new(config: &SdkConfig) -> Self {
        Builder::from(config).build()
    }
}

#[derive(Default)]
pub struct Builder {}
impl Builder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn endpoint_resolver<T>(mut self, _endpoint_resolver: T) -> Self {
        self
    }

    pub fn set_endpoint_resolver<T>(&mut self, _endpoint_resolver: T) -> &mut Self {
        self
    }

    pub fn region<T>(mut self, _region: T) -> Self {
        self
    }

    pub fn credentials_provider<T>(mut self, _credentials_provider: T) -> Self {
        self
    }

    pub fn set_credentials_provider<T>(&mut self, _credentials_provider: T) -> &mut Self {
        self
    }

    pub fn build(self) -> Config {
        Config {}
    }

    pub fn from<T>(_config: &T) -> Self {
        Self {}
    }
}

impl From<&SdkConfig> for Builder {
    fn from(_input: &SdkConfig) -> Self {
        Builder::default()
    }
}

impl From<&SdkConfig> for Config {
    fn from(_sdk_config: &SdkConfig) -> Self {
        Config {}
    }
}
