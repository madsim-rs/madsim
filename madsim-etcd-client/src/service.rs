use super::*;
use madsim::rand::{thread_rng, Rng};
use spin::Mutex;
use std::collections::BTreeMap;
use std::time::Duration;

#[derive(Debug)]
pub struct EtcdService {
    timeout_rate: f32,
    inner: Mutex<ServiceInner>,
}

impl EtcdService {
    pub fn new(timeout_rate: f32) -> Self {
        EtcdService {
            timeout_rate,
            inner: Mutex::new(ServiceInner::default()),
        }
    }

    pub async fn put(
        &self,
        key: Vec<u8>,
        value: Vec<u8>,
        options: PutOptions,
    ) -> Result<PutResponse> {
        self.timeout().await?;
        let rsp = self.inner.lock().put(key, value, options);
        Ok(rsp)
    }

    pub async fn get(&self, key: Vec<u8>, options: GetOptions) -> Result<GetResponse> {
        self.timeout().await?;
        let rsp = self.inner.lock().get(key, options);
        Ok(rsp)
    }

    pub async fn delete(&self, key: Vec<u8>, options: DeleteOptions) -> Result<DeleteResponse> {
        self.timeout().await?;
        let rsp = self.inner.lock().delete(key, options);
        Ok(rsp)
    }

    pub async fn txn(&self, txn: Txn) -> Result<TxnResponse> {
        self.timeout().await?;
        let rsp = self.inner.lock().txn(txn);
        Ok(rsp)
    }

    async fn timeout(&self) -> Result<()> {
        if thread_rng().gen_bool(self.timeout_rate as f64) {
            let t = thread_rng().gen_range(Duration::from_secs(5)..Duration::from_secs(15));
            madsim::time::sleep(t).await;
            tracing::warn!(?t, "etcdserver: request timed out");
            return Err(Error::GRpcStatus(tonic::Status::new(
                tonic::Code::Unavailable,
                "etcdserver: request timed out",
            )));
        }
        Ok(())
    }
}

#[derive(Debug, Default)]
struct ServiceInner {
    revision: i64,
    kv: BTreeMap<Vec<u8>, Vec<u8>>,
}

impl ServiceInner {
    fn header(&self) -> ResponseHeader {
        ResponseHeader {
            revision: self.revision,
        }
    }

    fn put(&mut self, key: Vec<u8>, value: Vec<u8>, _options: PutOptions) -> PutResponse {
        tracing::trace!(
            key = ?String::from_utf8_lossy(&key),
            value = ?String::from_utf8_lossy(&value),
            "put"
        );
        self.kv.insert(key, value);
        self.revision += 1;
        PutResponse {
            header: self.header(),
            prev_kv: None,
        }
    }

    fn get(&mut self, key: Vec<u8>, options: GetOptions) -> GetResponse {
        tracing::trace!(
            key = ?String::from_utf8_lossy(&key),
            ?options,
            "get"
        );
        if options.revision > 0 {
            todo!("get with revision");
        }
        let kvs = if options.prefix {
            let mut end = key.clone();
            *end.last_mut().unwrap() += 1;
            self.kv
                .range(key..end)
                .map(|(k, v)| KeyValue {
                    key: k.clone(),
                    value: v.clone(),
                })
                .collect()
        } else {
            self.kv
                .get(&key)
                .map(|v| KeyValue {
                    key: key.clone(),
                    value: v.clone(),
                })
                .into_iter()
                .collect()
        };
        GetResponse {
            header: self.header(),
            kvs,
        }
    }

    fn delete(&mut self, key: Vec<u8>, _options: DeleteOptions) -> DeleteResponse {
        tracing::trace!(
            key = ?String::from_utf8_lossy(&key),
            "delete"
        );
        let deleted = self.kv.remove(&key).map_or(0, |_| 1);
        self.revision += 1;
        DeleteResponse {
            header: self.header(),
            deleted,
        }
    }

    fn txn(&mut self, txn: Txn) -> TxnResponse {
        tracing::trace!(%txn, "transaction");
        let succeeded = txn.compare.iter().all(|cmp| {
            let value = self.kv.get(&cmp.key);
            match cmp.op {
                CompareOp::Equal => value == Some(&cmp.value),
                CompareOp::Greater => matches!(value, Some(v) if v > &cmp.value),
                CompareOp::Less => matches!(value, Some(v) if v < &cmp.value),
                CompareOp::NotEqual => value != Some(&cmp.value),
            }
        });

        let revision = self.revision;
        let mut op_responses = vec![];
        for op in if succeeded { txn.success } else { txn.failure } {
            let response = match op {
                TxnOp::Get { key, options } => TxnOpResponse::Get(self.get(key, options)),
                TxnOp::Put {
                    key,
                    value,
                    options,
                } => TxnOpResponse::Put(self.put(key, value, options)),
                TxnOp::Delete { key, options } => TxnOpResponse::Delete(self.delete(key, options)),
                TxnOp::Txn { txn: _txn } => todo!(),
            };
            op_responses.push(response);
        }
        self.revision = revision + 1;

        TxnResponse {
            header: self.header(),
            succeeded,
            op_responses,
        }
    }
}
