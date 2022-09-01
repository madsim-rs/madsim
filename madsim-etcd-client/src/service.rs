use super::kv::*;
use std::collections::BTreeMap;

#[derive(Debug, Default)]
pub struct EtcdService {
    revision: i64,
    kv: BTreeMap<Vec<u8>, Vec<u8>>,
}

impl EtcdService {
    fn header(&self) -> ResponseHeader {
        ResponseHeader {
            revision: self.revision,
        }
    }

    pub fn put(&mut self, key: Vec<u8>, value: Vec<u8>, _options: PutOptions) -> PutResponse {
        self.kv.insert(key, value);
        self.revision += 1;
        PutResponse {
            header: self.header(),
            prev_kv: None,
        }
    }

    pub fn get(&mut self, key: Vec<u8>, options: GetOptions) -> GetResponse {
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

    pub fn delete(&mut self, key: Vec<u8>, _options: DeleteOptions) -> DeleteResponse {
        let deleted = self.kv.remove(&key).map_or(0, |_| 1);
        self.revision += 1;
        DeleteResponse {
            header: self.header(),
            deleted,
        }
    }

    pub fn txn(&mut self, txn: Txn) -> TxnResponse {
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
