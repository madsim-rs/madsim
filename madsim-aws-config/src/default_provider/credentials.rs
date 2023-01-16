use std::time::Duration;

use aws_types::credentials::{self, future, ProvideCredentials};
use aws_types::Credentials;

use crate::meta::region::ProvideRegion;

#[cfg(any(feature = "rustls", feature = "native-tls"))]
pub async fn default_provider() -> impl ProvideCredentials {
    DefaultCredentialsChain::builder().build().await
}

#[derive(Debug)]
pub struct DefaultCredentialsChain {}

impl DefaultCredentialsChain {
    pub fn builder() -> Builder {
        Builder::default()
    }

    async fn credentials(&self) -> credentials::Result {
        Ok(Credentials {})
    }
}

impl ProvideCredentials for DefaultCredentialsChain {
    fn provide_credentials<'a>(&'a self) -> future::ProvideCredentials<'a>
    where
        Self: 'a,
    {
        future::ProvideCredentials::new(self.credentials())
    }
}

#[derive(Debug, Default)]
pub struct Builder {}

impl Builder {
    pub fn region(mut self, _region: impl ProvideRegion + 'static) -> Self {
        self
    }

    pub fn set_region(&mut self, _region: Option<impl ProvideRegion + 'static>) -> &mut Self {
        self
    }

    pub fn load_timeout(mut self, _timeout: Duration) -> Self {
        self
    }

    pub fn set_load_timeout(&mut self, _timeout: Option<Duration>) -> &mut Self {
        self
    }

    pub fn buffer_time(mut self, _buffer_time: Duration) -> Self {
        self
    }

    pub fn set_buffer_time(&mut self, _buffer_time: Option<Duration>) -> &mut Self {
        self
    }

    pub fn default_credential_expiration(mut self, _duration: Duration) -> Self {
        self
    }

    pub fn set_default_credential_expiration(&mut self, _duration: Option<Duration>) -> &mut Self {
        self
    }

    // pub fn with_custom_credential_source(
    //     mut self,
    //     name: impl Into<Cow<'static, str>>,
    //     provider: impl ProvideCredentials + 'static,
    // ) -> Self {
    //     self.profile_file_builder = self
    //         .profile_file_builder
    //         .with_custom_provider(name, provider);
    //     self
    // }

    pub fn profile_name(mut self, _name: &str) -> Self {
        self
    }

    // pub fn configure(mut self, config: ProviderConfig) -> Self {
    //     self.region_chain = self.region_chain.configure(&config);
    //     self.conf = Some(config);
    //     self
    // }

    pub async fn build(self) -> DefaultCredentialsChain {
        DefaultCredentialsChain {}
    }
}
