use super::{KeyValue, ResponseHeader, Result};
use futures_util::stream::Stream;
use madsim::net::Endpoint;
use std::{
    net::SocketAddr,
    pin::Pin,
    task::{Context, Poll},
};

/// Client for lease operations.
#[derive(Clone)]
pub struct LeaseClient {
    ep: Endpoint,
    server_addr: SocketAddr,
}

impl LeaseClient {
    /// Create a new [`LeaseClient`].
    pub(crate) fn new(ep: Endpoint) -> Self {
        LeaseClient {
            server_addr: ep.peer_addr().unwrap(),
            ep,
        }
    }

    /// Creates a lease which expires if the server does not receive a keepAlive
    /// within a given time to live period. All keys attached to the lease will be expired and
    /// deleted if the lease expires. Each expired key generates a delete event in the event history.
    #[inline]
    pub async fn grant(
        &mut self,
        ttl: i64,
        options: Option<LeaseGrantOptions>,
    ) -> Result<LeaseGrantResponse> {
        todo!()
    }

    /// Revokes a lease. All keys attached to the lease will expire and be deleted.
    #[inline]
    pub async fn revoke(&mut self, id: i64) -> Result<LeaseRevokeResponse> {
        todo!()
    }

    /// Keeps the lease alive by streaming keep alive requests from the client
    /// to the server and streaming keep alive responses from the server to the client.
    #[inline]
    pub async fn keep_alive(&mut self, id: i64) -> Result<(LeaseKeeper, LeaseKeepAliveStream)> {
        todo!()
    }

    /// Retrieves lease information.
    #[inline]
    pub async fn time_to_live(
        &mut self,
        id: i64,
        options: Option<LeaseTimeToLiveOptions>,
    ) -> Result<LeaseTimeToLiveResponse> {
        todo!()
    }

    /// Lists all existing leases.
    #[inline]
    pub async fn leases(&mut self) -> Result<LeaseLeasesResponse> {
        todo!()
    }
}

/// Options for `Grant` operation.
#[derive(Debug, Default, Clone)]
pub struct LeaseGrantOptions {
    id: i64,
    ttl: i64,
}

impl LeaseGrantOptions {
    /// Set ttl
    #[inline]
    const fn with_ttl(mut self, ttl: i64) -> Self {
        self.ttl = ttl;
        self
    }

    /// Set id
    #[inline]
    pub const fn with_id(mut self, id: i64) -> Self {
        self.id = id;
        self
    }

    /// Creates a [`LeaseGrantOptions`].
    #[inline]
    pub const fn new() -> Self {
        Self { ttl: 0, id: 0 }
    }
}

/// Response for `Grant` operation.
#[derive(Debug, Clone)]
pub struct LeaseGrantResponse {
    header: ResponseHeader,
    id: i64,
    ttl: i64,
}

impl LeaseGrantResponse {
    /// Get response header.
    #[inline]
    pub fn header(&self) -> Option<&ResponseHeader> {
        Some(&self.header)
    }

    /// TTL is the server chosen lease time-to-live in seconds
    #[inline]
    pub const fn ttl(&self) -> i64 {
        self.ttl
    }

    /// ID is the lease ID for the granted lease.
    #[inline]
    pub const fn id(&self) -> i64 {
        self.id
    }
}

/// Response for `Revoke` operation.
#[derive(Debug, Clone)]
pub struct LeaseRevokeResponse {
    pub(crate) header: ResponseHeader,
}

impl LeaseRevokeResponse {
    /// Gets response header.
    #[inline]
    pub fn header(&self) -> Option<&ResponseHeader> {
        Some(&self.header)
    }
}

/// Lease status.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LeaseStatus {
    id: i64,
}

impl LeaseStatus {
    /// Lease id.
    #[inline]
    pub const fn id(&self) -> i64 {
        self.id
    }
}

/// The lease keep alive handle.
#[derive(Debug)]
pub struct LeaseKeeper {
    id: i64,
}

impl LeaseKeeper {
    /// The lease id which user want to keep alive.
    #[inline]
    pub const fn id(&self) -> i64 {
        self.id
    }

    /// Sends a keep alive request and receive response
    #[inline]
    pub async fn keep_alive(&mut self) -> Result<()> {
        todo!()
    }
}

/// The lease keep alive response stream.
#[derive(Debug)]
pub struct LeaseKeepAliveStream {}

impl LeaseKeepAliveStream {
    /// Fetches the next message from this stream.
    #[inline]
    pub async fn message(&mut self) -> Result<Option<LeaseKeepAliveResponse>> {
        todo!()
    }
}

impl Stream for LeaseKeepAliveStream {
    type Item = Result<LeaseKeepAliveResponse>;

    #[inline]
    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        todo!()
    }
}

/// Response for `KeepAlive` operation.
#[derive(Debug, Clone)]
pub struct LeaseKeepAliveResponse {
    header: ResponseHeader,
    id: i64,
    ttl: i64,
}

impl LeaseKeepAliveResponse {
    /// Get response header.
    #[inline]
    pub fn header(&self) -> Option<&ResponseHeader> {
        Some(&self.header)
    }

    /// TTL is the new time-to-live for the lease.
    #[inline]
    pub const fn ttl(&self) -> i64 {
        self.ttl
    }

    /// ID is the lease ID for the keep alive request.
    #[inline]
    pub const fn id(&self) -> i64 {
        self.id
    }
}

/// Options for `TimeToLive` operation.
#[derive(Debug, Default, Clone)]
pub struct LeaseTimeToLiveOptions {
    id: i64,
    keys: bool,
}

impl LeaseTimeToLiveOptions {
    /// ID is the lease ID for the lease.
    #[inline]
    const fn with_id(mut self, id: i64) -> Self {
        self.id = id;
        self
    }

    /// Keys is true to query all the keys attached to this lease.
    #[inline]
    pub const fn with_keys(mut self) -> Self {
        self.keys = true;
        self
    }

    /// Creates a `LeaseTimeToLiveOptions`.
    #[inline]
    pub const fn new() -> Self {
        Self { id: 0, keys: false }
    }
}

/// Response for `TimeToLive` operation.
#[derive(Debug, Clone)]
pub struct LeaseTimeToLiveResponse {
    header: ResponseHeader,
    ttl: i64,
    id: i64,
    granted_ttl: i64,
    keys: Vec<Vec<u8>>,
}

impl LeaseTimeToLiveResponse {
    /// Get response header.
    #[inline]
    pub fn header(&self) -> Option<&ResponseHeader> {
        Some(&self.header)
    }

    /// TTL is the remaining TTL in seconds for the lease; the lease will expire in under TTL+1 seconds.
    #[inline]
    pub const fn ttl(&self) -> i64 {
        self.ttl
    }

    /// ID is the lease ID from the keep alive request.
    #[inline]
    pub const fn id(&self) -> i64 {
        self.id
    }

    /// GrantedTTL is the initial granted time in seconds upon lease creation/renewal.
    #[inline]
    pub const fn granted_ttl(&self) -> i64 {
        self.granted_ttl
    }

    /// Keys is the list of keys attached to this lease.
    #[inline]
    pub fn keys(&self) -> &[Vec<u8>] {
        &self.keys
    }
}

// Response for `Leases` operation.
#[derive(Debug, Clone)]
pub struct LeaseLeasesResponse {
    header: ResponseHeader,
    leases: Vec<LeaseStatus>,
}

impl LeaseLeasesResponse {
    /// Get response header.
    #[inline]
    pub fn header(&self) -> Option<&ResponseHeader> {
        Some(&self.header)
    }

    /// Get leases status
    #[inline]
    pub fn leases(&self) -> &[LeaseStatus] {
        &self.leases
    }
}
