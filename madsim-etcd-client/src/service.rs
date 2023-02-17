use super::*;
use madsim::rand::{random, thread_rng, Rng};
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};
use spin::Mutex;
use std::collections::btree_map::Entry;
use std::collections::{btree_map::Range, BTreeMap, HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;

#[derive(Debug)]
pub struct EtcdService {
    timeout_rate: f32,
    /// The maximum size of any request. 1.5 MiB
    max_request_bytes: usize,
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
            max_request_bytes: 1_572_864,
            inner,
        }
    }

    pub fn header(&self) -> ResponseHeader {
        self.inner.lock().header()
    }

    pub async fn put(&self, key: Key, value: Value, options: PutOptions) -> Result<PutResponse> {
        self.assert_request_size(key.len() + value.len())?;
        self.timeout().await?;
        let rsp = self.inner.lock().put(key, value, options)?;
        Ok(rsp)
    }

    pub async fn get(&self, key: Key, options: GetOptions) -> Result<GetResponse> {
        self.assert_request_size(key.len())?;
        self.timeout().await?;
        let rsp = self.inner.lock().get(key, options);
        Ok(rsp)
    }

    pub async fn delete(&self, key: Key, options: DeleteOptions) -> Result<DeleteResponse> {
        self.assert_request_size(key.len())?;
        self.timeout().await?;
        let rsp = self.inner.lock().delete(key, options);
        Ok(rsp)
    }

    pub async fn txn(&self, txn: Txn) -> Result<TxnResponse> {
        self.assert_request_size(txn.size())?;
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
        let rsp = self.inner.lock().lease_revoke(id)?;
        Ok(rsp)
    }

    pub async fn lease_keep_alive(&self, id: i64) -> Result<LeaseKeepAliveResponse> {
        self.timeout().await?;
        let rsp = self.inner.lock().lease_keep_alive(id)?;
        Ok(rsp)
    }

    pub async fn lease_time_to_live(&self, id: i64, keys: bool) -> Result<LeaseTimeToLiveResponse> {
        self.timeout().await?;
        let rsp = self.inner.lock().lease_time_to_live(id, keys)?;
        Ok(rsp)
    }

    pub async fn lease_leases(&self) -> Result<LeaseLeasesResponse> {
        self.timeout().await?;
        let rsp = self.inner.lock().lease_leases();
        Ok(rsp)
    }

    pub async fn campaign(&self, name: Key, value: Value, lease: i64) -> Result<CampaignResponse> {
        self.assert_request_size(name.len() + value.len())?;
        self.timeout().await?;
        loop {
            let mut rx = match self.inner.lock().campaign(&name, &value, lease)? {
                Ok(rsp) => return Ok(rsp),
                Err(rx) => rx,
            };
            rx.recv().await.expect("sender should not drop");
        }
    }

    pub async fn proclaim(&self, leader: LeaderKey, value: Value) -> Result<ProclaimResponse> {
        self.assert_request_size(leader.size() + value.len())?;
        self.timeout().await?;
        self.inner.lock().proclaim(leader, value)
    }

    pub async fn leader(&self, name: Key) -> Result<LeaderResponse> {
        self.assert_request_size(name.len())?;
        self.timeout().await?;
        self.inner.lock().leader(name)
    }

    pub async fn observe(&self, name: Key) -> Result<mpsc::Receiver<Event>> {
        self.assert_request_size(name.len())?;
        self.timeout().await?;
        self.inner.lock().observe(name)
    }

    pub async fn resign(&self, leader: LeaderKey) -> Result<ResignResponse> {
        self.assert_request_size(leader.size())?;
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

    fn assert_request_size(&self, size: usize) -> Result<()> {
        if size > self.max_request_bytes {
            return Err(Error::GRpcStatus(tonic::Status::new(
                tonic::Code::InvalidArgument,
                "etcdserver: request is too large",
            )));
        }
        Ok(())
    }
}

#[serde_as]
#[derive(Debug, Default, Serialize, Deserialize)]
struct ServiceInner {
    revision: i64,
    kv: BTreeMap<Key, KeyValue>,
    #[serde_as(as = "HashMap<DisplayFromStr, _>")]
    lease: HashMap<LeaseId, Lease>,
    #[serde(skip)]
    watcher: EventBus,
}

#[derive(Debug, Default)]
struct EventBus {
    list: Vec<(EventPattern, mpsc::Sender<Event>)>,
}

#[derive(Debug)]
enum EventPattern {
    Leader(Key),
    Prefix(Key),
}

impl EventPattern {
    fn is_match(&self, event: &Event) -> bool {
        match self {
            Self::Leader(prefix) => {
                event.kv.key.starts_with(prefix) && event.event_type == EventType::Delete
            }
            Self::Prefix(prefix) => event.kv.key.starts_with(prefix),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Event {
    pub event_type: EventType,
    pub kv: KeyValue,
}

impl Event {
    /// Returns a put event.
    fn put(kv: KeyValue) -> Self {
        Self {
            event_type: EventType::Put,
            kv,
        }
    }

    /// Returns a delete event.
    fn delete(prev_kv: KeyValue) -> Self {
        Self {
            event_type: EventType::Delete,
            kv: prev_kv,
        }
    }
}

impl EventBus {
    /// Subscribe a watcher.
    fn subscribe(&mut self, pattern: EventPattern, tx: mpsc::Sender<Event>) {
        tracing::trace!(?pattern, "subscribe");
        self.list.push((pattern, tx));
    }

    /// Publish an event.
    fn publish(&mut self, event: Event) {
        tracing::trace!(?event, "new event");
        self.list.retain(|(pattern, tx)| {
            if pattern.is_match(&event) {
                tx.try_send(event.clone()).is_ok()
            } else {
                true
            }
        });
    }
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

    fn put(&mut self, key: Key, value: Value, options: PutOptions) -> Result<PutResponse> {
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
            let lease = self
                .lease
                .get_mut(&options.lease)
                .ok_or_else(lease_not_found)?;
            lease.keys.insert(key.clone());
        }
        // remove key from the old lease
        if let Some(v) = &prev_value {
            if v.lease != 0 {
                let lease = self.lease.get_mut(&v.lease).expect("no lease");
                lease.keys.remove(&key);
            }
        }
        // update main key-value
        self.revision += 1;
        let kv = KeyValue {
            key: key.clone(),
            value,
            lease: options.lease,
            create_revision: prev_value
                .as_ref()
                .map_or(self.revision, |v| v.create_revision),
            modify_revision: self.revision,
        };
        *self.kv.entry(key).or_default() = kv.clone();
        self.watcher.publish(Event::put(kv));

        Ok(PutResponse {
            header: self.header(),
            prev_kv: if options.prev_kv { prev_value } else { None },
        })
    }

    fn get(&mut self, key: Key, options: GetOptions) -> GetResponse {
        tracing::trace!(?key, ?options, "get");
        if options.revision > 0 {
            todo!("get with revision");
        }
        let kvs = if options.prefix {
            self.get_prefix_range(key).map(|(_, v)| v.clone()).collect()
        } else {
            self.kv.get(&key).cloned().into_iter().collect()
        };
        GetResponse {
            header: self.header(),
            kvs,
        }
    }

    fn get_prefix_range(&self, key: Key) -> Range<'_, Key, KeyValue> {
        let mut end = key.clone();
        *end.last_mut().unwrap() += 1;
        self.kv.range(key..end)
    }

    fn delete(&mut self, key: Key, _options: DeleteOptions) -> DeleteResponse {
        tracing::trace!(?key, "delete");
        let prev_kv = self.kv.remove(&key);
        let deleted = prev_kv.is_some() as i64;
        if let Some(kv) = prev_kv {
            self.revision += 1;
            // remove key from the lease
            if kv.lease != 0 {
                let lease = self.lease.get_mut(&kv.lease).expect("no lease");
                lease.keys.remove(&key);
            }
            self.watcher.publish(Event::delete(kv));
        }
        DeleteResponse {
            header: self.header(),
            deleted,
        }
    }

    fn txn(&mut self, txn: Txn) -> TxnResponse {
        tracing::trace!(%txn, "transaction");
        let succeeded = txn.compare.iter().all(|cmp| {
            let value = self.kv.get(&cmp.key).map(|v| &v.value);
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
                } => TxnOpResponse::Put(self.put(key, value, options).expect("put failed in txn")),
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

    fn lease_revoke(&mut self, id: i64) -> Result<LeaseRevokeResponse> {
        tracing::trace!(id, "lease_revoke");
        let lease = self.lease.remove(&id).ok_or_else(lease_not_found)?;
        for key in lease.keys {
            tracing::trace!(?key, "delete");
            let kv = self.kv.remove(&key).expect("no key");
            self.watcher.publish(Event::delete(kv));
        }
        self.revision += 1;
        Ok(LeaseRevokeResponse {
            header: self.header(),
        })
    }

    fn lease_keep_alive(&mut self, id: i64) -> Result<LeaseKeepAliveResponse> {
        tracing::trace!(id, "lease_keep_alive");
        let lease = self.lease.get_mut(&id).ok_or_else(lease_not_found)?;
        let ttl = lease.granted_ttl;
        lease.ttl = ttl;
        self.revision += 1;
        Ok(LeaseKeepAliveResponse {
            header: self.header(),
            id,
            ttl,
        })
    }

    fn lease_time_to_live(&self, id: i64, keys: bool) -> Result<LeaseTimeToLiveResponse> {
        let lease = self.lease.get(&id).ok_or_else(lease_not_found)?;
        Ok(LeaseTimeToLiveResponse {
            header: self.header(),
            id,
            ttl: lease.ttl,
            granted_ttl: lease.granted_ttl,
            keys: if keys {
                lease.keys.iter().map(|k| k.to_vec()).collect()
            } else {
                vec![]
            },
        })
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
                    tracing::trace!(?key, "delete");
                    let kv = self.kv.remove(key).expect("no key");
                    self.watcher.publish(Event::delete(kv));
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

    fn campaign(
        &mut self,
        name: &Key,
        value: &Value,
        lease: i64,
    ) -> Result<std::result::Result<CampaignResponse, mpsc::Receiver<Event>>> {
        if self.get_prefix_range(name.clone()).next().is_some() {
            // the election name is occupied
            let (tx, rx) = mpsc::channel(1);
            self.watcher
                .subscribe(EventPattern::Leader(name.clone()), tx);
            return Ok(Err(rx));
        }
        tracing::trace!(?name, ?value, lease, "new leader");

        // key = format!("{name}/{lease:016x}")
        let mut key = name.clone();
        key.push(b'/');
        key.extend_from_slice(format!("{lease:016x}").as_bytes());

        let kv = KeyValue {
            key: key.clone(),
            value: value.clone(),
            lease,
            create_revision: self.revision,
            modify_revision: self.revision,
        };
        self.lease
            .get_mut(&lease)
            .ok_or_else(lease_not_found)?
            .keys
            .insert(key.clone());
        self.kv.insert(key.clone(), kv.clone());
        self.watcher.publish(Event::put(kv));
        self.revision += 1;

        Ok(Ok(CampaignResponse {
            header: self.header(),
            leader: LeaderKey {
                name: name.clone(),
                key,
                rev: self.revision,
                lease,
            },
        }))
    }

    fn proclaim(&mut self, leader: LeaderKey, value: Value) -> Result<ProclaimResponse> {
        tracing::trace!(name = ?leader.name, ?value, "proclaim");
        match self.kv.entry(leader.key) {
            Entry::Vacant(_) => return Err(session_expired()),
            Entry::Occupied(mut entry) => {
                self.revision += 1;
                entry.get_mut().value = value;
                entry.get_mut().modify_revision = self.revision;
                self.watcher.publish(Event::put(entry.get_mut().clone()));
            }
        }
        Ok(ProclaimResponse {
            header: self.header(),
        })
    }

    fn leader(&self, name: Key) -> Result<LeaderResponse> {
        Ok(LeaderResponse {
            header: self.header(),
            kv: self.get_prefix_range(name).next().map(|(_, v)| v.clone()),
        })
    }

    fn observe(&mut self, name: Key) -> Result<mpsc::Receiver<Event>> {
        tracing::trace!(?name, "observe");
        let (tx, rx) = mpsc::channel(10);
        self.watcher.subscribe(EventPattern::Prefix(name), tx);
        Ok(rx)
    }

    fn resign(&mut self, leader: LeaderKey) -> Result<ResignResponse> {
        tracing::trace!(name = ?leader.name, "resign");
        let kv = self.kv.remove(&leader.key).ok_or_else(session_expired)?;
        self.watcher.publish(Event::delete(kv));
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

fn lease_not_found() -> Error {
    Error::GRpcStatus(tonic::Status::new(
        tonic::Code::NotFound,
        "etcdserver: requested lease not found",
    ))
}

fn session_expired() -> Error {
    Error::ElectError("session expired".into())
}
