use crate::Credentials;
use std::error::Error;
use std::fmt::{self, Debug, Display, Formatter};
use std::sync::Arc;
use std::time::Duration;

#[derive(Debug)]
#[non_exhaustive]
pub enum CredentialsError {
    #[non_exhaustive]
    CredentialsNotLoaded {
        context: Box<dyn Error + Send + Sync + 'static>,
    },

    #[non_exhaustive]
    ProviderTimedOut(Duration),

    #[non_exhaustive]
    InvalidConfiguration {
        cause: Box<dyn Error + Send + Sync + 'static>,
    },

    #[non_exhaustive]
    ProviderError {
        cause: Box<dyn Error + Send + Sync + 'static>,
    },

    #[non_exhaustive]
    Unhandled {
        cause: Box<dyn Error + Send + Sync + 'static>,
    },
}

impl CredentialsError {
    pub fn not_loaded(context: impl Into<Box<dyn Error + Send + Sync + 'static>>) -> Self {
        CredentialsError::CredentialsNotLoaded {
            context: context.into(),
        }
    }

    pub fn unhandled(cause: impl Into<Box<dyn Error + Send + Sync + 'static>>) -> Self {
        Self::Unhandled {
            cause: cause.into(),
        }
    }

    pub fn provider_error(cause: impl Into<Box<dyn Error + Send + Sync + 'static>>) -> Self {
        Self::ProviderError {
            cause: cause.into(),
        }
    }

    pub fn invalid_configuration(cause: impl Into<Box<dyn Error + Send + Sync + 'static>>) -> Self {
        Self::InvalidConfiguration {
            cause: cause.into(),
        }
    }

    pub fn provider_timed_out(context: Duration) -> Self {
        Self::ProviderTimedOut(context)
    }
}

impl Display for CredentialsError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            CredentialsError::CredentialsNotLoaded { context } => {
                write!(f, "The credential provider was not enabled: {context}")
            }
            CredentialsError::ProviderTimedOut(d) => write!(
                f,
                "Credentials provider timed out after {} seconds",
                d.as_secs()
            ),
            CredentialsError::Unhandled { cause } => {
                write!(f, "Unexpected credentials error: {cause}")
            }
            CredentialsError::InvalidConfiguration { cause } => {
                write!(
                    f,
                    "The credentials provider was not properly configured: {cause}"
                )
            }
            CredentialsError::ProviderError { cause } => {
                write!(f, "An error occurred while loading credentials: {cause}")
            }
        }
    }
}

impl Error for CredentialsError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            CredentialsError::Unhandled { cause }
            | CredentialsError::ProviderError { cause }
            | CredentialsError::InvalidConfiguration { cause } => Some(cause.as_ref() as _),
            CredentialsError::CredentialsNotLoaded { context } => Some(context.as_ref() as _),
            _ => None,
        }
    }
}

pub type Result = std::result::Result<Credentials, CredentialsError>;

pub mod future {
    use aws_smithy_async::future::now_or_later::NowOrLater;
    use std::future::Future;
    use std::pin::Pin;
    use std::task::{Context, Poll};

    type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

    #[derive(Debug)]
    pub struct ProvideCredentials<'a>(NowOrLater<super::Result, BoxFuture<'a, super::Result>>);

    impl<'a> ProvideCredentials<'a> {
        pub fn new(future: impl Future<Output = super::Result> + Send + 'a) -> Self {
            ProvideCredentials(NowOrLater::new(Box::pin(future)))
        }

        pub fn ready(credentials: super::Result) -> Self {
            ProvideCredentials(NowOrLater::ready(credentials))
        }
    }

    impl Future for ProvideCredentials<'_> {
        type Output = super::Result;

        fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
            Pin::new(&mut self.0).poll(cx)
        }
    }
}

pub trait ProvideCredentials: Send + Sync + Debug {
    fn provide_credentials<'a>(&'a self) -> future::ProvideCredentials<'a>
    where
        Self: 'a;
}

impl ProvideCredentials for Credentials {
    fn provide_credentials<'a>(&'a self) -> future::ProvideCredentials<'a>
    where
        Self: 'a,
    {
        future::ProvideCredentials::ready(Ok(self.clone()))
    }
}

impl ProvideCredentials for Arc<dyn ProvideCredentials> {
    fn provide_credentials<'a>(&'a self) -> future::ProvideCredentials<'a>
    where
        Self: 'a,
    {
        self.as_ref().provide_credentials()
    }
}

#[derive(Clone, Debug)]
pub struct SharedCredentialsProvider(Arc<dyn ProvideCredentials>);

impl SharedCredentialsProvider {
    pub fn new(provider: impl ProvideCredentials + 'static) -> Self {
        Self(Arc::new(provider))
    }
}

impl AsRef<dyn ProvideCredentials> for SharedCredentialsProvider {
    fn as_ref(&self) -> &(dyn ProvideCredentials + 'static) {
        self.0.as_ref()
    }
}

impl From<Arc<dyn ProvideCredentials>> for SharedCredentialsProvider {
    fn from(provider: Arc<dyn ProvideCredentials>) -> Self {
        SharedCredentialsProvider(provider)
    }
}

impl ProvideCredentials for SharedCredentialsProvider {
    fn provide_credentials<'a>(&'a self) -> future::ProvideCredentials<'a>
    where
        Self: 'a,
    {
        self.0.provide_credentials()
    }
}
