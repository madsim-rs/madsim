use super::*;
use futures_util::future::poll_fn;
use madsim::rand::{random, thread_rng, Rng};
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};
use spin::Mutex;
use std::collections::btree_map::Entry;
use std::collections::{btree_map::Range, BTreeMap, HashMap, HashSet};
use std::sync::Arc;
use std::task::{Context, Poll, Waker};
use std::time::Duration;

#[derive(Debug)]
pub struct EtcdService {
    timeout_rate: f32,
    inner: Arc<Mutex<ServiceInner>>,
}

impl EtcdService {
    pub fn new(timeout_rate: f32, data: Option<String>) -> Self {
        let inner = Arc::new(Mutex::new(
            data.map_or_else(ServiceInner::default, |data| {
                toml::from_str(&data).expect("failed to deserialize dump")
            }),
        ));
        let weak = Arc::downgrade(&inner);
        madsim::task::spawn(async move {
            while let Some(inner) = weak.upgrade() {
                inner.lock().tick();
                drop(inner);
                madsim::time::sleep(Duration::from_secs(1)).await;
            }
        });
        EtcdService {
            timeout_rate,
            inner,
        }
    }

    pub async fn put(&self, key: Key, value: Value, options: PutOptions) -> Result<PutResponse> {
        self.timeout().await?;
        let rsp = self.inner.lock().put(key, value, options);
        Ok(rsp)
    }

    pub async fn get(&self, key: Key, options: GetOptions) -> Result<GetResponse> {
        self.timeout().await?;
        let rsp = self.inner.lock().get(key, options);
        Ok(rsp)
    }

    pub async fn delete(&self, key: Key, options: DeleteOptions) -> Result<DeleteResponse> {
        self.timeout().await?;
        let rsp = self.inner.lock().delete(key, options);
        Ok(rsp)
    }

    pub async fn txn(&self, txn: Txn) -> Result<TxnResponse> {
        self.timeout().await?;
        let rsp = self.inner.lock().txn(txn);
        Ok(rsp)
    }

    pub async fn lease_grant(&self, ttl: i64, id: i64) -> Result<LeaseGrantResponse> {
        self.timeout().await?;
        let rsp = self.inner.lock().lease_grant(ttl, id);
        Ok(rsp)
    }

    pub async fn lease_revoke(&self, id: i64) -> Result<LeaseRevokeResponse> {
        self.timeout().await?;
        let rsp = self.inner.lock().lease_revoke(id);
        Ok(rsp)
    }

    pub async fn lease_keep_alive(&self, id: i64) -> Result<LeaseKeepAliveResponse> {
        self.timeout().await?;
        let rsp = self.inner.lock().lease_keep_alive(id);
        Ok(rsp)
    }

    pub async fn lease_time_to_live(&self, id: i64, keys: bool) -> Result<LeaseTimeToLiveResponse> {
        self.timeout().await?;
        let rsp = self.inner.lock().lease_time_to_live(id, keys);
        Ok(rsp)
    }

    pub async fn lease_leases(&self) -> Result<LeaseLeasesResponse> {
        self.timeout().await?;
        let rsp = self.inner.lock().lease_leases();
        Ok(rsp)
    }

    pub async fn campaign(&self, name: Key, value: Value, lease: i64) -> Result<CampaignResponse> {
        self.timeout().await?;
        let rsp = poll_fn(|cx| self.inner.lock().poll_campaign(&name, &value, lease, cx)).await;
        Ok(rsp)
    }

    pub async fn proclaim(&self, leader: LeaderKey, value: Value) -> Result<ProclaimResponse> {
        self.timeout().await?;
        self.inner.lock().proclaim(leader, value)
    }

    pub async fn leader(&self, name: Key) -> Result<LeaderResponse> {
        self.timeout().await?;
        self.inner.lock().leader(name)
    }

    pub async fn resign(&self, leader: LeaderKey) -> Result<ResignResponse> {
        self.timeout().await?;
        self.inner.lock().resign(leader)
    }

    pub async fn status(&self) -> Result<StatusResponse> {
        self.timeout().await?;
        self.inner.lock().status()
    }

    pub async fn dump(&self) -> Result<String> {
        let inner = &*self.inner.lock();
        Ok(toml::to_string(inner).expect("failed to serialize dump"))
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

#[serde_as]
#[derive(Debug, Default, Serialize, Deserialize)]
struct ServiceInner {
    revision: i64,
    kv: BTreeMap<Key, (Value, LeaseId)>,
    #[serde_as(as = "HashMap<DisplayFromStr, _>")]
    lease: HashMap<LeaseId, Lease>,
    /// Waiters for election.
    #[serde(skip)]
    waiting_candidates: Vec<(Key, Waker)>,
}

type LeaseId = i64;
type Key = Bytes;
type Value = Bytes;

#[derive(Debug, Serialize, Deserialize)]
struct Lease {
    ttl: i64,
    granted_ttl: i64,
    keys: HashSet<Key>,
}

impl Lease {
    fn new(ttl: i64) -> Self {
        Lease {
            ttl,
            granted_ttl: ttl,
            keys: HashSet::new(),
        }
    }
}

impl ServiceInner {
    fn header(&self) -> ResponseHeader {
        ResponseHeader {
            revision: self.revision,
        }
    }

    fn put(&mut self, key: Key, value: Value, options: PutOptions) -> PutResponse {
        tracing::trace!(
            ?key,
            ?value,
            lease = if options.lease == 0 {
                None
            } else {
                Some(options.lease)
            },
            "put"
        );
        let prev_value = self.kv.get(&key).cloned();
        // add key to the new lease
        if options.lease != 0 {
            let lease = self.lease.get_mut(&options.lease).expect("no lease");
            lease.keys.insert(key.clone());
        }
        // remove key from the old lease
        if let Some((_, lease)) = prev_value {
            if lease != 0 {
                let lease = self.lease.get_mut(&lease).expect("no lease");
                lease.keys.remove(&key);
            }
        }
        // update main key-value
        *self.kv.entry(key.clone()).or_default() = (value, options.lease);

        self.revision += 1;
        PutResponse {
            header: self.header(),
            prev_kv: if options.prev_kv {
                prev_value.map(|(value, lease)| KeyValue { key, value, lease })
            } else {
                None
            },
        }
    }

    fn get(&mut self, key: Key, options: GetOptions) -> GetResponse {
        tracing::trace!(?key, ?options, "get");
        if options.revision > 0 {
            todo!("get with revision");
        }
        let kvs = if options.prefix {
            self.get_prefix_range(key)
                .map(|(k, (v, lease))| KeyValue {
                    key: k.clone(),
                    value: v.clone(),
                    lease: *lease,
                })
                .collect()
        } else {
            self.kv
                .get(&key)
                .map(|(v, lease)| KeyValue {
                    key: key.clone(),
                    value: v.clone(),
                    lease: *lease,
                })
                .into_iter()
                .collect()
        };
        GetResponse {
            header: self.header(),
            kvs,
        }
    }

    fn get_prefix_range(&self, key: Key) -> Range<'_, Key, (Value, LeaseId)> {
        let mut end = key.clone();
        *end.last_mut().unwrap() += 1;
        self.kv.range(key..end)
    }

    fn delete(&mut self, key: Key, _options: DeleteOptions) -> DeleteResponse {
        tracing::trace!(?key, "delete");
        let prev_kv = self.kv.remove(&key);
        if let Some((_, lease)) = &prev_kv {
            self.revision += 1;
            // remove key from the lease
            if *lease != 0 {
                let lease = self.lease.get_mut(lease).expect("no lease");
                lease.keys.remove(&key);
            }
            // TODO: notify one
            self.waiting_candidates.retain(|(prefix, waker)| {
                if key.starts_with(prefix) {
                    waker.wake_by_ref();
                    false
                } else {
                    true
                }
            });
        }
        DeleteResponse {
            header: self.header(),
            deleted: prev_kv.map_or(0, |_| 1),
        }
    }

    fn txn(&mut self, txn: Txn) -> TxnResponse {
        tracing::trace!(%txn, "transaction");
        let succeeded = txn.compare.iter().all(|cmp| {
            let value = self.kv.get(&cmp.key).map(|(v, _)| v);
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
                TxnOp::Txn { txn } => TxnOpResponse::Txn(self.txn(txn)),
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

    fn lease_grant(&mut self, ttl: i64, mut id: i64) -> LeaseGrantResponse {
        tracing::trace!(ttl, id, "lease_grant");
        // choose an ID if == 0
        if id == 0 {
            while self.lease.contains_key(&id) || id == 0 {
                id = random::<i64>().abs();
            }
        }
        let old = self.lease.insert(id, Lease::new(ttl));
        assert!(old.is_none(), "lease ID already exists");
        self.revision += 1;
        LeaseGrantResponse {
            header: self.header(),
            id,
            ttl,
        }
    }

    fn lease_revoke(&mut self, id: i64) -> LeaseRevokeResponse {
        tracing::trace!(id, "lease_revoke");
        let lease = self.lease.remove(&id).expect("no lease");
        for key in lease.keys {
            self.kv.remove(&key);
        }
        self.revision += 1;
        LeaseRevokeResponse {
            header: self.header(),
        }
    }

    fn lease_keep_alive(&mut self, id: i64) -> LeaseKeepAliveResponse {
        tracing::trace!(id, "lease_keep_alive");
        let lease = self.lease.get_mut(&id).expect("no lease");
        let ttl = lease.granted_ttl;
        lease.ttl = ttl;
        self.revision += 1;
        LeaseKeepAliveResponse {
            header: self.header(),
            id,
            ttl,
        }
    }

    fn lease_time_to_live(&self, id: i64, keys: bool) -> LeaseTimeToLiveResponse {
        let lease = self.lease.get(&id).expect("no lease");
        LeaseTimeToLiveResponse {
            header: self.header(),
            id,
            ttl: lease.ttl,
            granted_ttl: lease.granted_ttl,
            keys: if keys {
                lease.keys.iter().map(|k| k.to_vec()).collect()
            } else {
                vec![]
            },
        }
    }

    fn lease_leases(&self) -> LeaseLeasesResponse {
        LeaseLeasesResponse {
            header: self.header(),
            leases: self.lease.keys().map(|&id| LeaseStatus { id }).collect(),
        }
    }

    /// Clears expired lease. This should be called every seconds.
    fn tick(&mut self) {
        let origin_len = self.lease.len();
        self.lease.retain(|id, lease| {
            lease.ttl -= 1;
            if lease.ttl <= 0 {
                tracing::trace!(id, "lease expired");
                for key in &lease.keys {
                    self.kv.remove(key);
                }
                false
            } else {
                true
            }
        });
        if self.lease.len() != origin_len {
            self.revision += 1;
        }
    }

    fn poll_campaign(
        &mut self,
        name: &Key,
        value: &Value,
        lease: i64,
        cx: &mut Context<'_>,
    ) -> Poll<CampaignResponse> {
        if self.get_prefix_range(name.clone()).next().is_some() {
            // the election name is occupied
            self.waiting_candidates
                .push((name.clone(), cx.waker().clone()));
            return Poll::Pending;
        }

        // name = format!("{name}/{lease:016x}")
        let mut key = name.clone();
        key.push(b'/');
        key.extend_from_slice(format!("{lease:016x}").as_bytes());

        self.kv.insert(key.clone(), (value.clone(), lease));
        self.revision += 1;

        tracing::trace!(?name, ?value, lease, "new leader",);
        Poll::Ready(CampaignResponse {
            header: self.header(),
            leader: LeaderKey {
                name: name.clone(),
                key,
                rev: 0, // TODO: key revision
                lease,
            },
        })
    }

    fn proclaim(&mut self, leader: LeaderKey, value: Value) -> Result<ProclaimResponse> {
        tracing::trace!(name = ?leader.name, ?value, "proclaim");
        match self.kv.entry(leader.key) {
            Entry::Occupied(mut entry) => entry.get_mut().0 = value,
            Entry::Vacant(_) => return Err(Error::ElectError("session expired".into())),
        }
        self.revision += 1;
        Ok(ProclaimResponse {
            header: self.header(),
        })
    }

    fn leader(&self, name: Key) -> Result<LeaderResponse> {
        Ok(LeaderResponse {
            header: self.header(),
            kv: self
                .get_prefix_range(name)
                .next()
                .map(|(k, (v, lease))| KeyValue {
                    key: k.clone(),
                    value: v.clone(),
                    lease: *lease,
                }),
        })
    }

    fn resign(&mut self, leader: LeaderKey) -> Result<ResignResponse> {
        tracing::trace!(name = ?String::from_utf8_lossy(&leader.name), "resign");
        (self.kv.remove(&leader.key)).ok_or_else(|| Error::ElectError("session expired".into()))?;
        self.revision += 1;
        Ok(ResignResponse {
            header: self.header(),
        })
    }

    fn status(&mut self) -> Result<StatusResponse> {
        tracing::trace!("status");
        Ok(StatusResponse {
            header: self.header(),
        })
    }
}
