use std::borrow::Cow;
use std::fmt::{Display, Formatter, Result as FmtResult};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Region(Cow<'static, str>);

impl AsRef<str> for Region {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl Display for Region {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{}", self.0)
    }
}

impl Region {
    pub fn new(region: impl Into<Cow<'static, str>>) -> Self {
        Self(region.into())
    }

    pub const fn from_static(region: &'static str) -> Self {
        Self(Cow::Borrowed(region))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SigningRegion(Cow<'static, str>);

impl AsRef<str> for SigningRegion {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl From<Region> for SigningRegion {
    fn from(inp: Region) -> Self {
        SigningRegion(inp.0)
    }
}

impl From<&'static str> for SigningRegion {
    fn from(region: &'static str) -> Self {
        Self::from_static(region)
    }
}

impl SigningRegion {
    pub const fn from_static(region: &'static str) -> Self {
        SigningRegion(Cow::Borrowed(region))
    }
}
