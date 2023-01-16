use aws_types::region::Region;
use std::borrow::Cow;
use std::fmt::Debug;
use tracing::Instrument;

#[derive(Debug)]
pub struct RegionProviderChain {
    providers: Vec<Box<dyn ProvideRegion>>,
}

impl RegionProviderChain {
    pub async fn region(&self) -> Option<Region> {
        for provider in &self.providers {
            if let Some(region) = provider
                .region()
                .instrument(tracing::info_span!("load_region", provider = ?provider))
                .await
            {
                return Some(region);
            }
        }
        None
    }

    pub fn first_try(provider: impl ProvideRegion + 'static) -> Self {
        RegionProviderChain {
            providers: vec![Box::new(provider)],
        }
    }

    pub fn or_else(mut self, fallback: impl ProvideRegion + 'static) -> Self {
        self.providers.push(Box::new(fallback));
        self
    }

    pub fn default_provider() -> Self {
        Self::first_try(crate::default_provider::region::default_provider())
    }

    pub fn or_default_provider(mut self) -> Self {
        self.providers
            .push(Box::new(crate::default_provider::region::default_provider()));
        self
    }
}

impl ProvideRegion for Option<Region> {
    fn region(&self) -> future::ProvideRegion<'_> {
        future::ProvideRegion::ready(self.clone())
    }
}

impl ProvideRegion for RegionProviderChain {
    fn region(&self) -> future::ProvideRegion<'_> {
        future::ProvideRegion::new(RegionProviderChain::region(self))
    }
}

pub mod future {
    use std::future::Future;
    use std::pin::Pin;
    use std::task::{Context, Poll};

    use aws_smithy_async::future::now_or_later::NowOrLater;

    use aws_types::region::Region;

    type BoxFuture<'a> = Pin<Box<dyn Future<Output = Option<Region>> + Send + 'a>>;

    #[derive(Debug)]
    pub struct ProvideRegion<'a>(NowOrLater<Option<Region>, BoxFuture<'a>>);
    impl<'a> ProvideRegion<'a> {
        pub fn new(future: impl Future<Output = Option<Region>> + Send + 'a) -> Self {
            Self(NowOrLater::new(Box::pin(future)))
        }

        pub fn ready(region: Option<Region>) -> Self {
            Self(NowOrLater::ready(region))
        }
    }

    impl Future for ProvideRegion<'_> {
        type Output = Option<Region>;

        fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
            Pin::new(&mut self.0).poll(cx)
        }
    }
}

pub trait ProvideRegion: Send + Sync + Debug {
    fn region(&self) -> future::ProvideRegion<'_>;
}

impl ProvideRegion for Region {
    fn region(&self) -> future::ProvideRegion<'_> {
        future::ProvideRegion::ready(Some(self.clone()))
    }
}

impl<'a> ProvideRegion for &'a Region {
    fn region(&self) -> future::ProvideRegion<'_> {
        future::ProvideRegion::ready(Some((*self).clone()))
    }
}

impl ProvideRegion for Box<dyn ProvideRegion> {
    fn region(&self) -> future::ProvideRegion<'_> {
        self.as_ref().region()
    }
}

impl ProvideRegion for &'static str {
    fn region(&self) -> future::ProvideRegion<'_> {
        future::ProvideRegion::ready(Some(Region::new(Cow::Borrowed(*self))))
    }
}
