//! Server implementation and builder.

use std::{convert::Infallible, future::Future, net::SocketAddr};
use tower_service::Service;

/// A default batteries included `transport` server.
pub struct Server {}

impl Server {
    /// Create a new server builder that can configure a [`Server`].
    pub fn builder() -> Self {
        todo!()
    }
    /// Create a router with the `S` typed service as the first service.
    pub fn add_service<S>(&mut self, svc: S) -> Router
    where
        S: Service<(), Response = (), Error = Infallible>,
    {
        todo!()
    }
}

/// A stack based `Service` router.
pub struct Router {}

impl Router {
    /// Add a new service to this router.
    pub fn add_service<S>(&mut self, svc: S) -> Self {
        todo!()
    }
    /// Consume this [`Server`] creating a future that will execute the server
    /// on default executor.
    pub async fn serve(self, addr: SocketAddr) -> Result<(), super::Error> {
        todo!()
    }
    /// Consume this [`Server`] creating a future that will execute the server
    /// on default executor. And shutdown when the provided signal is received.
    pub async fn serve_with_shutdown(
        self,
        addr: SocketAddr,
        signal: impl Future<Output = ()>,
    ) -> Result<(), super::Error> {
        todo!()
    }
}
