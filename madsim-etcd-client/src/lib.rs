#![allow(unused_variables)]

mod error;
mod kv;

use std::time::Duration;

pub use self::error::{Error, Result};
pub use self::kv::*;

/// Asynchronous `etcd` client using v3 API.
#[derive(Clone)]
pub struct Client {
    kv: KvClient,
}

impl Client {
    /// Connect to `etcd` servers from given `endpoints`.
    pub async fn connect<E: AsRef<str>, S: AsRef<[E]>>(
        endpoints: S,
        options: Option<ConnectOptions>,
    ) -> Result<Self> {
        todo!()
    }

    /// Gets a KV client.
    pub fn kv_client(&self) -> KvClient {
        self.kv.clone()
    }
}

/// Options for [`Connect`] operation.
#[derive(Debug, Default, Clone)]
pub struct ConnectOptions {
    /// user is a pair values of name and password
    user: Option<(String, String)>,
    /// HTTP2 keep-alive: (keep_alive_interval, keep_alive_timeout)
    keep_alive: Option<(Duration, Duration)>,
}

impl ConnectOptions {
    /// Creates a `ConnectOptions`.
    #[inline]
    pub const fn new() -> Self {
        ConnectOptions {
            user: None,
            keep_alive: None,
        }
    }

    /// name is the identifier for the distributed shared lock to be acquired.
    #[inline]
    pub fn with_user(mut self, name: impl Into<String>, password: impl Into<String>) -> Self {
        self.user = Some((name.into(), password.into()));
        self
    }

    /// Enable HTTP2 keep-alive with `interval` and `timeout`.
    #[inline]
    pub fn with_keep_alive(mut self, interval: Duration, timeout: Duration) -> Self {
        self.keep_alive = Some((interval, timeout));
        self
    }
}

/// Transaction comparision.
#[derive(Debug, Clone)]
pub struct Compare {}

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
        todo!()
    }
}
