//! Client implementation and builder.

use super::Error;
use std::{fmt, net::IpAddr, sync::Arc, time::Duration};
use tonic::{codegen::StdError, transport::Uri};

/// Channel builder.
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
        Self {
            uri: Uri::from_static(s),
            timeout: None,
        }
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
        let addr: IpAddr = host.parse().map_err(|_| Error::new_invalid_uri())?;
        let port = self.uri.port_u16().ok_or_else(Error::new_invalid_uri)?;
        let ep = madsim::net::Endpoint::connect((addr, port))
            .await
            .map_err(Error::from_source)?;
        // NOTE: no actual connection here
        Ok(Channel { ep: Arc::new(ep) })
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
        todo!()
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
