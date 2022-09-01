use std::net::SocketAddr;

use super::*;

/// Client for KV operations.
#[derive(Clone)]
pub struct KvClient {
    ep: Endpoint,
    server_addr: SocketAddr,
}

impl KvClient {
    /// Create a new [`KvClient`].
    pub(crate) fn new(ep: Endpoint, addr: SocketAddr) -> Self {
        KvClient {
            ep,
            server_addr: addr,
        }
    }

    /// Puts the given key into the key-value store.
    /// A put request increments the revision of the key-value store
    /// and generates one event in the event history.
    #[inline]
    pub async fn put(
        &mut self,
        key: impl Into<Vec<u8>>,
        value: impl Into<Vec<u8>>,
        options: Option<PutOptions>,
    ) -> Result<PutResponse> {
        let req = Request::Put {
            key: key.into(),
            value: value.into(),
            options: options.unwrap_or_default(),
        };
        let (tx, mut rx) = self.ep.connect1(self.server_addr).await?;
        tx.send(Box::new(req)).await?;
        let rsp = *rx.recv().await?.downcast::<PutResponse>().unwrap();
        Ok(rsp)
    }

    /// Gets the key or a range of keys from the store.
    #[inline]
    pub async fn get(
        &mut self,
        key: impl Into<Vec<u8>>,
        options: Option<GetOptions>,
    ) -> Result<GetResponse> {
        let req = Request::Get {
            key: key.into(),
            options: options.unwrap_or_default(),
        };
        let (tx, mut rx) = self.ep.connect1(self.server_addr).await?;
        tx.send(Box::new(req)).await?;
        let rsp = *rx.recv().await?.downcast::<GetResponse>().unwrap();
        Ok(rsp)
    }

    /// Deletes the given key or a range of keys from the key-value store.
    #[inline]
    pub async fn delete(
        &mut self,
        key: impl Into<Vec<u8>>,
        options: Option<DeleteOptions>,
    ) -> Result<DeleteResponse> {
        let req = Request::Delete {
            key: key.into(),
            options: options.unwrap_or_default(),
        };
        let (tx, mut rx) = self.ep.connect1(self.server_addr).await?;
        tx.send(Box::new(req)).await?;
        let rsp = *rx.recv().await?.downcast::<DeleteResponse>().unwrap();
        Ok(rsp)
    }

    /// Compacts the event history in the etcd key-value store. The key-value
    /// store should be periodically compacted or the event history will continue to grow
    /// indefinitely.
    #[inline]
    pub async fn compact(
        &mut self,
        revision: i64,
        options: Option<CompactionOptions>,
    ) -> Result<CompactionResponse> {
        todo!()
    }

    /// Processes multiple operations in a single transaction.
    /// A txn request increments the revision of the key-value store
    /// and generates events with the same revision for every completed operation.
    /// It is not allowed to modify the same key several times within one txn.
    #[inline]
    pub async fn txn(&mut self, txn: Txn) -> Result<TxnResponse> {
        let req = Request::Txn { txn };
        let (tx, mut rx) = self.ep.connect1(self.server_addr).await?;
        tx.send(Box::new(req)).await?;
        let rsp = *rx.recv().await?.downcast::<TxnResponse>().unwrap();
        Ok(rsp)
    }
}

/// Options for `Put` operation.
#[derive(Debug, Default, Clone)]
pub struct PutOptions();

/// Response for `Put` operation.
#[derive(Debug, Clone)]
pub struct PutResponse {
    pub(crate) header: ResponseHeader,
    pub(crate) prev_kv: Option<KeyValue>,
}

impl PutResponse {
    /// Get response header.
    #[inline]
    pub fn header(&self) -> Option<&ResponseHeader> {
        Some(&self.header)
    }

    /// If prev_kv is set in the request, the previous key-value pair will be returned.
    #[inline]
    pub fn prev_key(&self) -> Option<&KeyValue> {
        self.prev_kv.as_ref()
    }
}

/// Options for `Get` operation.
#[derive(Debug, Default, Clone)]
pub struct GetOptions {
    revision: i64,
    prefix: bool,
}

impl GetOptions {
    /// Creates a `GetOptions`.
    #[inline]
    pub const fn new() -> Self {
        GetOptions {
            revision: 0,
            prefix: true,
        }
    }

    /// The point-in-time of the key-value store to use for the range.
    /// If revision is less or equal to zero, the range is over the newest key-value store.
    /// If the revision has been compacted, ErrCompacted is returned as a response.
    #[inline]
    pub const fn with_revision(mut self, revision: i64) -> Self {
        self.revision = revision;
        self
    }

    /// Gets all keys prefixed with key.
    #[inline]
    pub fn with_prefix(mut self) -> Self {
        self.prefix = true;
        self
    }
}

/// Response for `Get` operation.
#[derive(Debug, Clone)]
pub struct GetResponse {
    pub(crate) header: ResponseHeader,
    pub(crate) kvs: Vec<KeyValue>,
}

impl GetResponse {
    /// Get response header.
    #[inline]
    pub fn header(&self) -> Option<&ResponseHeader> {
        Some(&self.header)
    }

    /// The list of key-value pairs matched by the `Get` request.
    /// kvs is empty when count is requested.
    #[inline]
    pub fn kvs(&self) -> &[KeyValue] {
        &self.kvs
    }
}

/// General `etcd` response header.
#[derive(Debug, Clone)]
pub struct ResponseHeader {
    pub(crate) revision: i64,
}

impl ResponseHeader {
    /// The key-value store revision when the request was applied.
    #[inline]
    pub const fn revision(&self) -> i64 {
        self.revision
    }
}

/// Options for `Delete` operation.
#[derive(Debug, Default, Clone)]
pub struct DeleteOptions {}

/// Response for `Delete` operation.
#[derive(Debug, Clone)]
pub struct DeleteResponse {
    pub(crate) header: ResponseHeader,
    pub(crate) deleted: i64,
}

impl DeleteResponse {
    /// Get response header.
    #[inline]
    pub fn header(&self) -> Option<&ResponseHeader> {
        Some(&self.header)
    }

    /// The number of keys deleted by the delete request.
    #[inline]
    pub const fn deleted(&self) -> i64 {
        self.deleted
    }
}

/// Options for `Compact` operation.
#[derive(Debug, Default, Clone)]
pub struct CompactionOptions();

/// Response for `Compact` operation.
#[derive(Debug, Clone)]
pub struct CompactionResponse();

/// Transaction of multiple operations.
#[derive(Debug, Default, Clone)]
pub struct Txn {
    pub(crate) compare: Vec<Compare>,
    pub(crate) success: Vec<TxnOp>,
    pub(crate) failure: Vec<TxnOp>,
    c_when: bool,
    c_then: bool,
    c_else: bool,
}

impl Txn {
    /// Creates a new transaction.
    #[inline]
    pub const fn new() -> Self {
        Self {
            compare: vec![],
            success: vec![],
            failure: vec![],
            c_when: false,
            c_then: false,
            c_else: false,
        }
    }

    /// Takes a list of comparison. If all comparisons passed in succeed,
    /// the operations passed into `and_then()` will be executed. Or the operations
    /// passed into `or_else()` will be executed.
    #[inline]
    pub fn when(mut self, compares: impl Into<Vec<Compare>>) -> Self {
        assert!(!self.c_when, "cannot call when twice");
        assert!(!self.c_then, "cannot call when after and_then");
        assert!(!self.c_else, "cannot call when after or_else");

        self.c_when = true;
        self.compare = compares.into();
        self
    }

    /// Takes a list of operations. The operations list will be executed, if the
    /// comparisons passed in `when()` succeed.
    #[inline]
    pub fn and_then(mut self, operations: impl Into<Vec<TxnOp>>) -> Self {
        assert!(!self.c_then, "cannot call and_then twice");
        assert!(!self.c_else, "cannot call and_then after or_else");

        self.c_then = true;
        self.success = operations.into();
        self
    }

    /// Takes a list of operations. The operations list will be executed, if the
    /// comparisons passed in `when()` fail.
    #[inline]
    pub fn or_else(mut self, operations: impl Into<Vec<TxnOp>>) -> Self {
        assert!(!self.c_else, "cannot call or_else twice");

        self.c_else = true;
        self.failure = operations.into();
        self
    }
}

/// Transaction comparision.
#[derive(Debug, Clone)]
pub struct Compare {
    pub(crate) key: Vec<u8>,
    pub(crate) value: Vec<u8>,
    pub(crate) op: CompareOp,
}

///  Logical comparison operation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(i32)]
pub enum CompareOp {
    Equal = 0,
    Greater = 1,
    Less = 2,
    NotEqual = 3,
}

impl Compare {
    /// Compares the value of the given key.
    #[inline]
    pub fn value(key: impl Into<Vec<u8>>, cmp: CompareOp, value: impl Into<Vec<u8>>) -> Self {
        Compare {
            key: key.into(),
            value: value.into(),
            op: cmp,
        }
    }
}

/// Transaction operation.
#[derive(Debug, Clone)]
pub enum TxnOp {
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
}

impl TxnOp {
    /// `Put` operation.
    #[inline]
    pub fn put(
        key: impl Into<Vec<u8>>,
        value: impl Into<Vec<u8>>,
        options: Option<PutOptions>,
    ) -> Self {
        TxnOp::Put {
            key: key.into(),
            value: value.into(),
            options: options.unwrap_or_default(),
        }
    }

    /// `Get` operation.
    #[inline]
    pub fn get(key: impl Into<Vec<u8>>, options: Option<GetOptions>) -> Self {
        TxnOp::Get {
            key: key.into(),
            options: options.unwrap_or_default(),
        }
    }

    /// `Delete` operation.
    #[inline]
    pub fn delete(key: impl Into<Vec<u8>>, options: Option<DeleteOptions>) -> Self {
        TxnOp::Delete {
            key: key.into(),
            options: options.unwrap_or_default(),
        }
    }

    /// `Txn` operation.
    #[inline]
    pub fn txn(txn: Txn) -> Self {
        TxnOp::Txn { txn }
    }
}

/// Response for `Txn` operation.
#[derive(Debug, Clone)]
pub struct TxnResponse {
    pub(crate) header: ResponseHeader,
    pub(crate) succeeded: bool,
    pub(crate) op_responses: Vec<TxnOpResponse>,
}

#[derive(Debug, Clone)]
pub enum TxnOpResponse {
    Put(PutResponse),
    Get(GetResponse),
    Delete(DeleteResponse),
    Txn(TxnResponse),
}

impl TxnResponse {
    /// Transaction response header.
    #[inline]
    pub fn header(&self) -> Option<&ResponseHeader> {
        Some(&self.header)
    }

    /// Returns `true` if the compare evaluated to true or `false` otherwise.
    #[inline]
    pub const fn succeeded(&self) -> bool {
        self.succeeded
    }

    /// Returns responses of transaction operations.
    #[inline]
    pub fn op_responses(&self) -> Vec<TxnOpResponse> {
        self.op_responses.clone()
    }
}

/// Key-value pair.
#[derive(Debug, Clone)]
pub struct KeyValue {
    pub(crate) key: Vec<u8>,
    pub(crate) value: Vec<u8>,
}

impl KeyValue {
    /// The key in bytes. An empty key is not allowed.
    #[inline]
    pub fn key(&self) -> &[u8] {
        &self.key
    }

    /// The value held by the key, in bytes.
    #[inline]
    pub fn value(&self) -> &[u8] {
        &self.value
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
}
