use madsim::net::{Endpoint, Payload};
use spin::Mutex;
use std::{io::Result, net::SocketAddr, sync::Arc};

use super::{service::EtcdService, Request};

/// A simulated etcd server.
pub struct SimServer;

impl SimServer {
    pub async fn serve(addr: SocketAddr) -> Result<()> {
        let ep = Endpoint::bind(addr).await?;
        let service = Arc::new(Mutex::new(EtcdService::default()));
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
                    } => Box::new(service.lock().put(key, value, options)),
                    Request::Get { key, options } => Box::new(service.lock().get(key, options)),
                    Request::Delete { key, options } => {
                        Box::new(service.lock().delete(key, options))
                    }
                    Request::Txn { txn } => Box::new(service.lock().txn(txn)),
                };
                tx.send(response).await?;
                Ok(()) as Result<()>
            });
        }
    }
}
