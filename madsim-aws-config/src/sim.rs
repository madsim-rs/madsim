pub mod default_provider;
pub mod meta;
pub mod provider_config;
pub mod sts;

pub use aws_smithy_types::retry::RetryConfig;
pub use aws_smithy_types::timeout;

pub use aws_types::SdkConfig;

pub fn from_env() -> ConfigLoader {
    ConfigLoader::default()
}

pub async fn load_from_env() -> aws_types::SdkConfig {
    from_env().load().await
}

pub use loader::ConfigLoader;

mod loader {

    use aws_smithy_types::retry::RetryConfig;
    use aws_types::credentials::ProvideCredentials;
    use aws_types::endpoint::ResolveAwsEndpoint;
    use aws_types::SdkConfig;

    use crate::meta::region::ProvideRegion;

    #[derive(Default, Debug)]
    pub struct ConfigLoader {}

    impl ConfigLoader {
        pub fn region(mut self, _region: impl ProvideRegion + 'static) -> Self {
            self
        }

        pub fn retry_config(mut self, _retry_config: RetryConfig) -> Self {
            self
        }

        pub fn credentials_provider(
            mut self,
            _credentials_provider: impl ProvideCredentials + 'static,
        ) -> Self {
            self
        }

        pub fn endpoint_resolver(
            mut self,
            _endpoint_resolver: impl ResolveAwsEndpoint + 'static,
        ) -> Self {
            self
        }

        pub async fn load(self) -> SdkConfig {
            SdkConfig {}
        }
    }
}
