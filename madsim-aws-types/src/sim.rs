pub mod credentials;
pub use credentials::Credentials;
pub mod endpoint;
pub mod region;
pub mod sdk_config;

pub use aws_smithy_client::http_connector;
pub use sdk_config::SdkConfig;

use std::borrow::Cow;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SigningService(Cow<'static, str>);
impl AsRef<str> for SigningService {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl SigningService {
    pub fn from_static(service: &'static str) -> Self {
        SigningService(Cow::Borrowed(service))
    }
}

impl From<String> for SigningService {
    fn from(service: String) -> Self {
        SigningService(Cow::Owned(service))
    }
}

impl From<&'static str> for SigningService {
    fn from(service: &'static str) -> Self {
        Self::from_static(service)
    }
}
