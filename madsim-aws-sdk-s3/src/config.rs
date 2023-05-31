use std::fmt::Debug;

pub use aws_sdk_s3::config::Credentials;
pub use aws_sdk_s3::config::Region;
use aws_types::SdkConfig;

#[derive(Debug)]
pub struct Config {
    pub(crate) endpoint_url: String,
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
    endpoint_url: Option<String>,
    credentials: Option<Credentials>,
}

impl Builder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn endpoint_url(mut self, endpoint_url: impl Into<String>) -> Self {
        self.endpoint_url = Some(endpoint_url.into());
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
            endpoint_url: self.endpoint_url.expect("endpoint_url must be set"),
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
