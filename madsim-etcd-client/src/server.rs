use madsim::net::{Endpoint, Payload};
use std::{io::Result, net::SocketAddr, sync::Arc};

use super::{election::*, kv::*, service::EtcdService};

/// A simulated etcd server.
#[derive(Default, Clone)]
pub struct SimServer {
    timeout_rate: f32,
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

    /// Consume this [`SimServer`] creating a future that will execute the server.
    pub async fn serve(self, addr: SocketAddr) -> Result<()> {
        let ep = Endpoint::bind(addr).await?;
        let service = Arc::new(EtcdService::new(self.timeout_rate));
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
                    Request::Campaign { name, value, lease } => todo!(),
                    Request::Proclaim { leader, value } => todo!(),
                    Request::Leader { name } => todo!(),
                    Request::Observe { name } => todo!(),
                    Request::Resign { leader } => todo!(),
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
    Campaign {
        name: Vec<u8>,
        value: Vec<u8>,
        lease: i64,
    },
    Proclaim {
        leader: Option<LeaderKey>,
        value: Vec<u8>,
    },
    Leader {
        name: Vec<u8>,
    },
    Observe {
        name: Vec<u8>,
    },
    Resign {
        leader: Option<LeaderKey>,
    },
}
