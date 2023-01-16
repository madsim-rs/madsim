use crate::region::Region;

#[derive(Debug, Clone)]
pub struct SdkConfig {}

#[derive(Debug, Default)]
pub struct Builder {}

impl Builder {
    pub fn region(mut self, _region: impl Into<Option<Region>>) -> Self {
        self
    }

    pub fn set_region(&mut self, _region: impl Into<Option<Region>>) -> &mut Self {
        self
    }

    pub fn build(self) -> SdkConfig {
        SdkConfig {}
    }
}

impl SdkConfig {
    pub fn builder() -> Builder {
        Builder::default()
    }
}
