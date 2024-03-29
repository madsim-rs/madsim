mod bytes;
mod election;
mod error;
mod kv;
mod lease;
mod maintenance;
mod server;
mod service;
mod watch;

use madsim::net::Endpoint;
use std::net::SocketAddr;
use std::time::Duration;

pub use self::bytes::Bytes;
pub use self::election::*;
pub use self::error::{Error, Result};
pub use self::kv::*;
pub use self::lease::*;
pub use self::maintenance::*;
pub use self::server::SimServer;
pub use self::watch::*;

use self::server::Request;

/// Asynchronous `etcd` client using v3 API.
#[derive(Clone)]
pub struct Client {
    ep: Endpoint,
    server_addr: SocketAddr,
}

impl Client {
    /// Connect to `etcd` servers from given `endpoints`.
    pub async fn connect<E: AsRef<str>, S: AsRef<[E]>>(
        endpoints: S,
        _options: Option<ConnectOptions>,
    ) -> Result<Self> {
        let addr = endpoints.as_ref()[0].as_ref();
        let ep = Endpoint::connect(addr).await?;
        let server_addr = ep.peer_addr().unwrap();
        Ok(Client { ep, server_addr })
    }

    /// Gets a KV client.
    #[inline]
    pub fn kv_client(&self) -> KvClient {
        KvClient::new(self.ep.clone())
    }

    /// Gets a lease client.
    #[inline]
    pub fn lease_client(&self) -> LeaseClient {
        LeaseClient::new(self.ep.clone())
    }

    /// Gets a election client.
    #[inline]
    pub fn election_client(&self) -> ElectionClient {
        ElectionClient::new(self.ep.clone())
    }

    /// Gets a maintenance client.
    #[inline]
    pub fn maintenance_client(&self) -> MaintenanceClient {
        MaintenanceClient::new(self.ep.clone())
    }

    /// Dump the data of the etcd server.
    #[inline]
    pub async fn dump(&mut self) -> Result<String> {
        let req = Request::Dump;
        let (tx, mut rx) = self.ep.connect1(self.server_addr).await?;
        tx.send(Box::new(req)).await?;
        *rx.recv().await?.downcast::<Result<String>>().unwrap()
    }
}

/// Options for [`Connect`] operation.
#[derive(Debug, Default, Clone)]
pub struct ConnectOptions {
    /// user is a pair values of name and password
    user: Option<(String, String)>,
    /// HTTP2 keep-alive: (keep_alive_interval, keep_alive_timeout)
    keep_alive: Option<(Duration, Duration)>,
}

impl ConnectOptions {
    /// Creates a `ConnectOptions`.
    #[inline]
    pub const fn new() -> Self {
        ConnectOptions {
            user: None,
            keep_alive: None,
        }
    }

    /// name is the identifier for the distributed shared lock to be acquired.
    #[inline]
    pub fn with_user(mut self, name: impl Into<String>, password: impl Into<String>) -> Self {
        self.user = Some((name.into(), password.into()));
        self
    }

    /// Enable HTTP2 keep-alive with `interval` and `timeout`.
    #[inline]
    pub fn with_keep_alive(mut self, interval: Duration, timeout: Duration) -> Self {
        self.keep_alive = Some((interval, timeout));
        self
    }
}

/// General `etcd` response header.
#[derive(Debug, Clone)]
pub struct ResponseHeader {
    pub(crate) revision: i64,
}

impl ResponseHeader {
    /// The key-value store revision when the request was applied.
    #[inline]
    pub const fn revision(&self) -> i64 {
        self.revision
    }
}
