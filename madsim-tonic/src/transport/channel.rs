//! Client implementation and builder.

use super::Error;
use std::{fmt, net::IpAddr, sync::Arc, time::Duration};
use tonic::{
    codegen::{Bytes, StdError},
    transport::Uri,
};

/// Channel builder.
#[derive(Debug, Clone)]
pub struct Endpoint {
    uri: Uri,
    timeout: Option<Duration>,
}

impl Endpoint {
    // FIXME: determine if we want to expose this or not. This is really
    // just used in codegen for a shortcut.
    #[doc(hidden)]
    pub fn new<D>(dst: D) -> Result<Self, Error>
    where
        D: TryInto<Self>,
        D::Error: Into<StdError>,
    {
        let me = dst.try_into().map_err(|e| Error::from_source(e.into()))?;
        Ok(me)
    }

    /// Convert an [`Endpoint`] from a static string.
    pub fn from_static(s: &'static str) -> Self {
        Self::from(Uri::from_static(s))
    }

    /// Convert an [`Endpoint`] from shared bytes.
    pub fn from_shared(s: impl Into<Bytes>) -> Result<Self, Error> {
        let uri = Uri::from_maybe_shared(s.into()).map_err(|e| Error::new_invalid_uri().with(e))?;
        Ok(Self::from(uri))
    }

    /// Apply a timeout to connecting to the uri.
    ///
    /// Defaults to no timeout.
    pub fn connect_timeout(mut self, dur: Duration) -> Self {
        self.timeout = Some(dur);
        self
    }

    /// Create a channel from this config.
    pub async fn connect(&self) -> Result<Channel, Error> {
        let host = self.uri.host().ok_or_else(Error::new_invalid_uri)?;
        let addr: IpAddr = host.parse().map_err(|e| Error::new_invalid_uri().with(e))?;
        let port = self.uri.port_u16().ok_or_else(Error::new_invalid_uri)?;
        let ep = madsim::net::Endpoint::connect((addr, port))
            .await
            .map_err(Error::from_source)?;
        // NOTE: no actual connection here
        Ok(Channel { ep: Arc::new(ep) })
    }
}

impl From<Uri> for Endpoint {
    fn from(uri: Uri) -> Self {
        Self { uri, timeout: None }
    }
}

impl TryFrom<&'static str> for Endpoint {
    type Error = Error;

    fn try_from(value: &'static str) -> Result<Self, Self::Error> {
        Ok(Endpoint::from_static(value))
    }
}

impl TryFrom<String> for Endpoint {
    type Error = Error;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Endpoint::from_shared(value)
    }
}

/// A default batteries included `transport` channel.
#[derive(Clone)]
pub struct Channel {
    pub(crate) ep: Arc<madsim::net::Endpoint>,
}

impl fmt::Debug for Channel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Channel").finish()
    }
}
