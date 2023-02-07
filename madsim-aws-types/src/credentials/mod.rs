mod credentials_impl;
mod provider;

pub use credentials_impl::Credentials;
pub use provider::future;
pub use provider::CredentialsError;
pub use provider::ProvideCredentials;
pub use provider::Result;
pub use provider::SharedCredentialsProvider;
