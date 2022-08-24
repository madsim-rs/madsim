use super::*;

/// Client for KV operations.
#[derive(Clone)]
pub struct KvClient {}

impl KvClient {
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
        todo!()
    }

    /// Gets the key or a range of keys from the store.
    #[inline]
    pub async fn get(
        &mut self,
        key: impl Into<Vec<u8>>,
        options: Option<GetOptions>,
    ) -> Result<GetResponse> {
        todo!()
    }

    /// Deletes the given key or a range of keys from the key-value store.
    #[inline]
    pub async fn delete(
        &mut self,
        key: impl Into<Vec<u8>>,
        options: Option<DeleteOptions>,
    ) -> Result<DeleteResponse> {
        todo!()
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
        todo!()
    }
}

/// Options for `Put` operation.
#[derive(Debug, Default, Clone)]
pub struct PutOptions();

/// Response for `Put` operation.
#[derive(Debug, Clone)]
pub struct PutResponse();

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
    header: ResponseHeader,
    kvs: Vec<KeyValue>,
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
    revision: i64,
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
pub struct DeleteResponse();

/// Options for `Compact` operation.
#[derive(Debug, Default, Clone)]
pub struct CompactionOptions();

/// Response for `Compact` operation.
#[derive(Debug, Clone)]
pub struct CompactionResponse();

/// Transaction of multiple operations.
#[derive(Debug, Default, Clone)]
pub struct Txn {
    compare: Vec<Compare>,
    success: Vec<TxnOp>,
    failure: Vec<TxnOp>,
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
    header: ResponseHeader,
    succeeded: bool,
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
}

/// Key-value pair.
#[derive(Debug, Clone)]
pub struct KeyValue {
    key: Vec<u8>,
    value: Vec<u8>,
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
