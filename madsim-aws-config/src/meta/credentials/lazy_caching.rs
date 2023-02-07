use std::time::Duration;

use aws_types::credentials::{future, ProvideCredentials};
use aws_types::Credentials;

const DEFAULT_LOAD_TIMEOUT: Duration = Duration::from_secs(5);
const DEFAULT_CREDENTIAL_EXPIRATION: Duration = Duration::from_secs(15 * 60);
const DEFAULT_BUFFER_TIME: Duration = Duration::from_secs(10);

#[derive(Debug)]
pub struct LazyCachingCredentialsProvider {}

impl LazyCachingCredentialsProvider {
    fn new() -> Self {
        LazyCachingCredentialsProvider {}
    }

    pub fn builder() -> builder::Builder {
        builder::Builder::new()
    }
}

impl ProvideCredentials for LazyCachingCredentialsProvider {
    fn provide_credentials<'a>(&'a self) -> future::ProvideCredentials<'_>
    where
        Self: 'a,
    {
        future::ProvideCredentials::new(async move { Ok(Credentials {}) })
    }
}

pub use builder::Builder;

mod builder {
    use std::time::Duration;

    use aws_smithy_async::rt::sleep::AsyncSleep;
    use aws_types::credentials::ProvideCredentials;

    use super::LazyCachingCredentialsProvider;
    use crate::provider_config::ProviderConfig;

    #[derive(Debug, Default)]
    pub struct Builder {}

    impl Builder {
        pub fn new() -> Self {
            Default::default()
        }

        pub fn configure(mut self, _config: &ProviderConfig) -> Self {
            self
        }

        pub fn load(mut self, _loader: impl ProvideCredentials + 'static) -> Self {
            self
        }

        pub fn set_load(
            &mut self,
            _loader: Option<impl ProvideCredentials + 'static>,
        ) -> &mut Self {
            self
        }

        pub fn sleep(mut self, _sleep: impl AsyncSleep + 'static) -> Self {
            self
        }

        pub fn set_sleep(&mut self, _sleep: Option<impl AsyncSleep + 'static>) -> &mut Self {
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

        pub fn set_default_credential_expiration(
            &mut self,
            _duration: Option<Duration>,
        ) -> &mut Self {
            self
        }

        pub fn build(self) -> LazyCachingCredentialsProvider {
            LazyCachingCredentialsProvider {}
        }
    }
}
