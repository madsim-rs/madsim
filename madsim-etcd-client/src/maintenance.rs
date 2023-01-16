use super::{server::Request, ResponseHeader, Result};
use madsim::net::Endpoint;
use std::net::SocketAddr;

/// Client for Maintenance operations.
#[derive(Clone)]
pub struct MaintenanceClient {
    ep: Endpoint,
    server_addr: SocketAddr,
}

impl MaintenanceClient {
    /// Create a new [`MaintenanceClient`].
    pub(crate) fn new(ep: Endpoint) -> Self {
        MaintenanceClient {
            server_addr: ep.peer_addr().unwrap(),
            ep,
        }
    }

    /// Get status of a member.
    #[inline]
    pub async fn status(&mut self) -> Result<StatusResponse> {
        let req = Request::Status;
        let (tx, mut rx) = self.ep.connect1(self.server_addr).await?;
        tx.send(Box::new(req)).await?;
        *rx.recv().await?.downcast().unwrap()
    }
}

/// Response for `Status` operation.
#[derive(Debug, Clone)]
pub struct StatusResponse {
    pub(crate) header: ResponseHeader,
}

impl StatusResponse {
    /// Gets response header.
    #[inline]
    pub fn header(&self) -> Option<&ResponseHeader> {
        Some(&self.header)
    }
}
