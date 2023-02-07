use aws_types::region::Region;

use crate::meta::region::ProvideRegion;
use crate::provider_config::ProviderConfig;

pub fn default_provider() -> impl ProvideRegion {
    Builder::default().build()
}

#[derive(Debug)]
pub struct DefaultRegionChain {
    region: Region,
}

impl DefaultRegionChain {
    pub async fn region(&self) -> Option<Region> {
        Some(Region::new("default"))
    }

    pub fn builder() -> Builder {
        Builder::default()
    }
}

#[derive(Debug, Default)]
pub struct Builder {}

impl Builder {
    pub fn configure(mut self, _configuration: &ProviderConfig) -> Self {
        self
    }

    pub fn profile_name(mut self, _name: &str) -> Self {
        self
    }

    pub fn build(self) -> DefaultRegionChain {
        DefaultRegionChain {
            region: Region::new("default"),
        }
    }
}

impl ProvideRegion for DefaultRegionChain {
    fn region(&self) -> crate::meta::region::future::ProvideRegion<'_> {
        ProvideRegion::region(&self.region)
    }
}
