use madsim::net::{Endpoint, Payload};
use std::{io::Result, net::SocketAddr, sync::Arc};

use super::{election::*, kv::*, service::EtcdService};

/// A simulated etcd server.
#[derive(Default, Clone)]
pub struct SimServer {
    timeout_rate: f32,
    load: Option<String>,
}

impl SimServer {
    /// Create a new server builder that can configure a [`SimServer`].
    pub fn builder() -> Self {
        SimServer::default()
    }

    /// Set the rate of `etcdserver: request timed out`.
    pub fn timeout_rate(mut self, rate: f32) -> Self {
        assert!((0.0..=1.0).contains(&rate));
        self.timeout_rate = rate;
        self
    }

    /// Load data from dump.
    pub fn load(mut self, data: String) -> Self {
        self.load = Some(data);
        self
    }

    /// Consume this [`SimServer`] creating a future that will execute the server.
    pub async fn serve(self, addr: SocketAddr) -> Result<()> {
        let ep = Endpoint::bind(addr).await?;
        let service = Arc::new(EtcdService::new(self.timeout_rate, self.load));
        loop {
            let (tx, mut rx, _) = ep.accept1().await?;
            let service = service.clone();
            madsim::task::spawn(async move {
                let request = *rx.recv().await?.downcast::<Request>().unwrap();
                let response: Payload = match request {
                    Request::Put {
                        key,
                        value,
                        options,
                    } => Box::new(service.put(key, value, options).await),
                    Request::Get { key, options } => Box::new(service.get(key, options).await),
                    Request::Delete { key, options } => {
                        Box::new(service.delete(key, options).await)
                    }
                    Request::Txn { txn } => Box::new(service.txn(txn).await),
                    Request::LeaseGrant { ttl, id } => Box::new(service.lease_grant(ttl, id).await),
                    Request::LeaseRevoke { id } => Box::new(service.lease_revoke(id).await),
                    Request::LeaseKeepAlive { id } => Box::new(service.lease_keep_alive(id).await),
                    Request::LeaseTimeToLive { id, keys } => {
                        Box::new(service.lease_time_to_live(id, keys).await)
                    }
                    Request::LeaseLeases => Box::new(service.lease_leases().await),
                    Request::Campaign { name, value, lease } => {
                        Box::new(service.campaign(name, value, lease).await)
                    }
                    Request::Proclaim { leader, value } => {
                        Box::new(service.proclaim(leader, value).await)
                    }
                    Request::Leader { name } => Box::new(service.leader(name).await),
                    Request::Observe { name: _ } => todo!(),
                    Request::Resign { leader } => Box::new(service.resign(leader).await),
                    Request::Dump => Box::new(service.dump().await),
                };
                tx.send(response).await?;
                Ok(()) as Result<()>
            });
        }
    }
}

/// A request to etcd server.
#[derive(Debug)]
pub(crate) enum Request {
    // kv API
    Put {
        key: Vec<u8>,
        value: Vec<u8>,
        options: PutOptions,
    },
    Get {
        key: Vec<u8>,
        options: GetOptions,
    },
    Delete {
        key: Vec<u8>,
        options: DeleteOptions,
    },
    Txn {
        txn: Txn,
    },

    // lease API
    LeaseGrant {
        ttl: i64,
        id: i64,
    },
    LeaseRevoke {
        id: i64,
    },
    LeaseKeepAlive {
        id: i64,
    },
    LeaseTimeToLive {
        id: i64,
        keys: bool,
    },
    LeaseLeases,

    // election API
    Campaign {
        name: Vec<u8>,
        value: Vec<u8>,
        lease: i64,
    },
    Proclaim {
        leader: LeaderKey,
        value: Vec<u8>,
    },
    Leader {
        name: Vec<u8>,
    },
    Observe {
        #[allow(dead_code)]
        name: Vec<u8>,
    },
    Resign {
        leader: LeaderKey,
    },

    // internal API
    Dump,
}
