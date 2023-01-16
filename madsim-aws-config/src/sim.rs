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

    // use aws_smithy_async::rt::sleep::AsyncSleep;
    // use aws_smithy_client::http_connector::HttpConnector;
    use aws_smithy_types::retry::RetryConfig;
    // use aws_smithy_types::timeout;
    use aws_types::credentials::ProvideCredentials;
    use aws_types::endpoint::ResolveAwsEndpoint;
    use aws_types::SdkConfig;

    use crate::meta::region::ProvideRegion;
    // use crate::provider_config::ProviderConfig;

    #[derive(Default, Debug)]
    pub struct ConfigLoader {}

    impl ConfigLoader {
        pub fn region(mut self, _region: impl ProvideRegion + 'static) -> Self {
            self
        }

        pub fn retry_config(mut self, _retry_config: RetryConfig) -> Self {
            self
        }

        // pub fn timeout_config(mut self, _timeout_config: timeout::Config) -> Self {
        //     self
        // }

        // pub fn sleep_impl(mut self, _sleep: impl AsyncSleep + 'static) -> Self {
        //     self
        // }

        // pub fn http_connector(mut self, _http_connector: HttpConnector) -> Self {
        //     self
        // }

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

        // pub fn configure(mut self, _provider_config: ProviderConfig) -> Self {
        //     self
        // }

        pub async fn load(self) -> SdkConfig {
            SdkConfig {}
        }
    }
}
