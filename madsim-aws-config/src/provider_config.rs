use std::fmt::Debug;

#[derive(Clone, Debug, Default)]
pub struct ProviderConfig {}

impl ProviderConfig {
    pub fn without_region() -> Self {
        Self::default()
    }

    pub fn empty() -> Self {
        ProviderConfig {}
    }
}
