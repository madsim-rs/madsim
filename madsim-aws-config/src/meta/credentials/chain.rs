use std::borrow::Cow;

use aws_types::credentials::{self, future, CredentialsError, ProvideCredentials};
use tracing::Instrument;

#[derive(Debug)]
pub struct CredentialsProviderChain {
    providers: Vec<(Cow<'static, str>, Box<dyn ProvideCredentials>)>,
}

impl CredentialsProviderChain {
    pub fn first_try(
        name: impl Into<Cow<'static, str>>,
        provider: impl ProvideCredentials + 'static,
    ) -> Self {
        CredentialsProviderChain {
            providers: vec![(name.into(), Box::new(provider))],
        }
    }

    pub fn or_else(
        mut self,
        name: impl Into<Cow<'static, str>>,
        provider: impl ProvideCredentials + 'static,
    ) -> Self {
        self.providers.push((name.into(), Box::new(provider)));
        self
    }

    #[cfg(any(feature = "rustls", feature = "native-tls"))]
    pub async fn or_default_provider(self) -> Self {
        self.or_else(
            "DefaultProviderChain",
            crate::default_provider::credentials::default_provider().await,
        )
    }

    #[cfg(any(feature = "rustls", feature = "native-tls"))]
    pub async fn default_provider() -> Self {
        Self::first_try(
            "DefaultProviderChain",
            crate::default_provider::credentials::default_provider().await,
        )
    }

    async fn credentials(&self) -> credentials::Result {
        for (name, provider) in &self.providers {
            let span = tracing::debug_span!("load_credentials", provider = %name);
            match provider.provide_credentials().instrument(span).await {
                Ok(credentials) => {
                    tracing::info!(provider = %name, "loaded credentials");
                    return Ok(credentials);
                }
                Err(CredentialsError::CredentialsNotLoaded { context, .. }) => {
                    tracing::info!(provider = %name, context = %context, "provider in chain did not provide credentials");
                }
                Err(e) => {
                    tracing::warn!(provider = %name, error = %e, "provider failed to provide credentials");
                    return Err(e);
                }
            }
        }
        Err(CredentialsError::not_loaded(
            "no providers in chain provided credentials",
        ))
    }
}

impl ProvideCredentials for CredentialsProviderChain {
    fn provide_credentials<'a>(&'a self) -> future::ProvideCredentials<'_>
    where
        Self: 'a,
    {
        future::ProvideCredentials::new(self.credentials())
    }
}
