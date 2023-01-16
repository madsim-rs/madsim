use aws_types::credentials;
use aws_types::credentials::ProvideCredentials;
use std::fmt::{self, Debug, Formatter};
use std::future::Future;
use std::marker::PhantomData;

#[derive(Copy, Clone)]
pub struct ProvideCredentialsFn<'c, T> {
    f: T,
    phantom: PhantomData<&'c T>,
}

impl<T> Debug for ProvideCredentialsFn<'_, T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "ProvideCredentialsFn")
    }
}

impl<'c, T, F> ProvideCredentials for ProvideCredentialsFn<'c, T>
where
    T: Fn() -> F + Send + Sync + 'c,
    F: Future<Output = credentials::Result> + Send + 'static,
{
    fn provide_credentials<'a>(&'a self) -> credentials::future::ProvideCredentials<'a>
    where
        Self: 'a,
    {
        credentials::future::ProvideCredentials::new((self.f)())
    }
}

pub fn provide_credentials_fn<'c, T, F>(f: T) -> ProvideCredentialsFn<'c, T>
where
    T: Fn() -> F + Send + Sync + 'c,
    F: Future<Output = credentials::Result> + Send + 'static,
{
    ProvideCredentialsFn {
        f,
        phantom: Default::default(),
    }
}
