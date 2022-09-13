use std::{
    fmt::{Debug, Formatter},
    sync::Arc,
};

use aws_endpoint::ResolveAwsEndpoint;
use aws_types::{
    credentials::{ProvideCredentials, SharedCredentialsProvider},
    region::Region,
    SdkConfig,
};

pub struct Config {
    pub(crate) endpoint_resolver: Arc<dyn ResolveAwsEndpoint>,
    pub(crate) region: Option<Region>,
    pub(crate) credentials_provider: SharedCredentialsProvider,
}
impl Debug for Config {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut config = f.debug_struct("Config");
        config.finish()
    }
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
    endpoint_resolver: Option<Arc<dyn ResolveAwsEndpoint>>,
    region: Option<Region>,
    credentials_provider: Option<SharedCredentialsProvider>,
}
impl Builder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn endpoint_resolver(
        mut self,
        endpoint_resolver: impl ResolveAwsEndpoint + 'static,
    ) -> Self {
        self.endpoint_resolver = Some(Arc::new(endpoint_resolver));
        self
    }

    pub fn set_endpoint_resolver(
        &mut self,
        endpoint_resolver: Option<Arc<dyn ResolveAwsEndpoint>>,
    ) -> &mut Self {
        self.endpoint_resolver = endpoint_resolver;
        self
    }

    pub fn region(mut self, region: impl Into<Option<Region>>) -> Self {
        self.region = region.into();
        self
    }

    pub fn credentials_provider(
        mut self,
        credentials_provider: impl ProvideCredentials + 'static,
    ) -> Self {
        self.credentials_provider = Some(SharedCredentialsProvider::new(credentials_provider));
        self
    }

    pub fn set_credentials_provider(
        &mut self,
        credentials_provider: Option<SharedCredentialsProvider>,
    ) -> &mut Self {
        self.credentials_provider = credentials_provider;
        self
    }

    pub fn build(self) -> Config {
        Config {
            endpoint_resolver: self
                .endpoint_resolver
                .unwrap_or_else(|| Arc::new(crate::aws_endpoint::endpoint_resolver())),
            region: self.region,
            credentials_provider: self.credentials_provider.unwrap_or_else(|| {
                SharedCredentialsProvider::new(crate::no_credentials::NoCredentials)
            }),
        }
    }
}

impl From<&SdkConfig> for Builder {
    fn from(input: &SdkConfig) -> Self {
        let mut builder = Builder::default();
        builder = builder.region(input.region().cloned());
        builder.set_endpoint_resolver(input.endpoint_resolver().clone());
        builder.set_credentials_provider(input.credentials_provider().cloned());
        builder
    }
}

impl From<&SdkConfig> for Config {
    fn from(sdk_config: &SdkConfig) -> Self {
        Builder::from(sdk_config).build()
    }
}
