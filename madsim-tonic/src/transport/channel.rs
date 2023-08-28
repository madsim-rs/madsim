//! Client implementation and builder.

use super::Error;
use madsim::rand::Rng;
use std::{
    collections::HashMap,
    fmt,
    hash::Hash,
    io,
    net::SocketAddr,
    str::FromStr,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::sync::mpsc::{channel, Receiver, Sender};
use tonic::{
    codegen::{http::HeaderValue, Bytes, StdError},
    transport::Uri,
};
use tower::discover::Change;

/// Channel builder.
#[derive(Debug, Clone)]
pub struct Endpoint {
    uri: Uri,
    timeout: Option<Duration>,
    connect_timeout: Option<Duration>,
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

    /// Apply a timeout to each request.
    pub fn timeout(self, dur: Duration) -> Self {
        Endpoint {
            timeout: Some(dur),
            ..self
        }
    }

    /// Apply a timeout to connecting to the uri.
    ///
    /// Defaults to no timeout.
    pub fn connect_timeout(self, dur: Duration) -> Self {
        Endpoint {
            connect_timeout: Some(dur),
            ..self
        }
    }

    /// Create a channel from this config.
    pub async fn connect(&self) -> Result<Channel, Error> {
        if let Some(dur) = self.connect_timeout {
            madsim::time::timeout(dur, self.connect_inner())
                .await
                .map_err(Error::from_source)?
        } else {
            self.connect_inner().await
        }
    }

    // Connect without timeout.
    async fn connect_inner(&self) -> Result<Channel, Error> {
        // check if the endpoint is available
        let _ep = self.connect_ep().await?;
        Ok(Channel {
            ep: MultiEndpoint::new_one(self.clone()),
            timeout: self.timeout,
        })
    }

    /// Connect to a madsim Endpoint.
    async fn connect_ep(&self) -> Result<madsim::net::Endpoint, Error> {
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

        Ok(ep)
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
        Self {
            uri,
            timeout: None,
            connect_timeout: None,
        }
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

impl FromStr for Endpoint {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::try_from(s.to_string())
    }
}

/// Default buffer size of `balance_channel`.
const DEFAULT_BUFFER_SIZE: usize = 1024;

/// A default batteries included `transport` channel.
#[derive(Clone)]
pub struct Channel {
    pub(crate) ep: MultiEndpoint,
    pub(crate) timeout: Option<Duration>,
}

impl Channel {
    /// Balance a list of [`Endpoint`]'s.
    ///
    /// This creates a [`Channel`] that will load balance across all the
    /// provided endpoints.
    pub fn balance_list(list: impl Iterator<Item = Endpoint>) -> Self {
        let (channel, tx) = Self::balance_channel(DEFAULT_BUFFER_SIZE);
        list.for_each(|endpoint| {
            tx.try_send(Change::Insert(endpoint.uri.clone(), endpoint))
                .unwrap();
        });

        channel
    }

    /// Balance a list of [`Endpoint`]'s.
    ///
    /// This creates a [`Channel`] that will listen to a stream of change events and will add or remove provided endpoints.
    pub fn balance_channel<K>(capacity: usize) -> (Self, Sender<Change<K, Endpoint>>)
    where
        K: Hash + Eq + Send + Clone + 'static,
    {
        let (tx, rx) = channel(capacity);
        let channel = Self {
            ep: MultiEndpoint::new_multi(rx),
            timeout: None,
        };
        (channel, tx)
    }
}

impl fmt::Debug for Channel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Channel").finish()
    }
}

#[derive(Clone)]
pub(crate) struct MultiEndpoint {
    balance: Arc<dyn Balance>,
}

impl MultiEndpoint {
    /// Creates with multiple endpoints
    pub(crate) fn new_multi<K>(rx: Receiver<Change<K, Endpoint>>) -> Self
    where
        K: Hash + Eq + Send + Clone + 'static,
    {
        Self {
            balance: Arc::new(DynamicEp::new(HashMap::new(), Some(rx))),
        }
    }

    /// Creates with one endpoint
    pub(crate) fn new_one(ep: Endpoint) -> Self {
        Self {
            balance: Arc::new(DynamicEp::new([((), ep)].into_iter().collect(), None)),
        }
    }

    pub(crate) async fn connect1(
        &self,
    ) -> io::Result<(madsim::net::Sender, madsim::net::Receiver)> {
        let ep = self.balance.get_one().ok_or(io::Error::new(
            io::ErrorKind::AddrNotAvailable,
            "no endpoints available",
        ))?;
        let madsim_ep = ep
            .connect_ep()
            .await
            .map_err(|e| io::Error::new(io::ErrorKind::ConnectionRefused, e.to_string()))?;
        let addr = madsim_ep.peer_addr().unwrap();
        madsim_ep.connect1(addr).await
    }
}

/// Dynamically monitor changes of endpoints
pub(crate) struct DynamicEp<K> {
    eps: Mutex<HashMap<K, Endpoint>>,
    rx: Option<Mutex<Receiver<Change<K, Endpoint>>>>,
}

impl<K> DynamicEp<K>
where
    K: Hash + Eq + Send + Clone + 'static,
{
    pub(crate) fn new(
        eps: HashMap<K, Endpoint>,
        rx: Option<Receiver<Change<K, Endpoint>>>,
    ) -> Self {
        Self {
            eps: Mutex::new(eps),
            rx: rx.map(Mutex::new),
        }
    }
}

impl<K> Balance for DynamicEp<K>
where
    K: Hash + Eq + Send + Clone + 'static,
{
    fn get_one(&self) -> Option<Endpoint> {
        let mut eps = self.eps.lock().unwrap();
        if let Some(rx) = self.rx.as_ref() {
            let mut rx_l = rx.lock().unwrap();
            while let Ok(change) = rx_l.try_recv() {
                match change {
                    Change::Insert(k, ep) => eps.insert(k, ep),
                    Change::Remove(k) => eps.remove(&k),
                };
            }
        }
        let len = eps.len();
        if len == 0 {
            return None;
        }
        let n = madsim::rand::thread_rng().gen_range(0..len);
        eps.values().nth(n).cloned()
    }
}

/// Balance among endpoints
pub(crate) trait Balance: Send + Sync {
    /// Get a random endpoint.
    fn get_one(&self) -> Option<Endpoint>;
}
