//! Client implementation and builder.

use super::Error;
use std::{fmt, net::SocketAddr, sync::Arc, time::Duration};
use tonic::{
    codegen::{http::HeaderValue, Bytes, StdError},
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
        let host_port = (self.uri.authority())
            .ok_or_else(Error::new_invalid_uri)?
            .as_str();
        let addr: SocketAddr = madsim::net::lookup_host(host_port)
            .await
            .map_err(|e| Error::new_invalid_uri().with(e))?
            .next()
            .ok_or_else(Error::new_invalid_uri)?;
        let ep = madsim::net::Endpoint::connect(addr)
            .await
            .map_err(Error::from_source)?;

        // handshake
        ep.connect1(addr).await.map_err(Error::from_source)?;

        Ok(Channel { ep: Arc::new(ep) })
    }

    /// Set a custom user-agent header.
    pub fn user_agent<T>(self, _user_agent: T) -> Result<Self, Error>
    where
        T: TryInto<HeaderValue>,
    {
        // ignore this setting
        Ok(self)
    }

    /// Set a custom origin.
    pub fn origin(self, _origin: Uri) -> Self {
        // ignore this setting
        self
    }

    /// Set whether TCP keepalive messages are enabled on accepted connections.
    pub fn tcp_keepalive(self, _tcp_keepalive: Option<Duration>) -> Self {
        // ignore this setting
        self
    }

    /// Apply a concurrency limit to each request.
    pub fn concurrency_limit(self, _limit: usize) -> Self {
        // ignore this setting
        self
    }

    /// Apply a rate limit to each request.
    pub fn rate_limit(self, _limit: u64, _duration: Duration) -> Self {
        // ignore this setting
        self
    }

    /// Sets the `SETTINGS_INITIAL_WINDOW_SIZE` option for HTTP2
    /// stream-level flow control.
    pub fn initial_stream_window_size(self, _sz: impl Into<Option<u32>>) -> Self {
        // ignore this setting
        self
    }

    /// Sets the max connection-level flow control for HTTP2
    pub fn initial_connection_window_size(self, _sz: impl Into<Option<u32>>) -> Self {
        // ignore this setting
        self
    }

    /// Set the value of `TCP_NODELAY` option for accepted connections. Enabled by default.
    pub fn tcp_nodelay(self, _enabled: bool) -> Self {
        // ignore this setting
        self
    }

    /// Set http2 KEEP_ALIVE_INTERVAL. Uses `hyper`'s default otherwise.
    pub fn http2_keep_alive_interval(self, _interval: Duration) -> Self {
        // ignore this setting
        self
    }

    /// Set http2 KEEP_ALIVE_TIMEOUT. Uses `hyper`'s default otherwise.
    pub fn keep_alive_timeout(self, _duration: Duration) -> Self {
        // ignore this setting
        self
    }

    /// Set http2 KEEP_ALIVE_WHILE_IDLE. Uses `hyper`'s default otherwise.
    pub fn keep_alive_while_idle(self, _enabled: bool) -> Self {
        // ignore this setting
        self
    }

    /// Sets whether to use an adaptive flow control. Uses `hyper`'s default otherwise.
    pub fn http2_adaptive_window(self, _enabled: bool) -> Self {
        // ignore this setting
        self
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
