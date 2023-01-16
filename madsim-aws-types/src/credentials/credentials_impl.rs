use std::fmt::Debug;
use std::time::SystemTime;

#[derive(Clone, Eq, PartialEq, Debug, Default)]
pub struct Credentials {}

impl Credentials {
    pub fn new(
        _access_key_id: impl Into<String>,
        _secret_access_key: impl Into<String>,
        _session_token: Option<String>,
        _expires_after: Option<SystemTime>,
        _provider_name: &'static str,
    ) -> Self {
        Credentials {}
    }

    pub fn from_keys(
        _access_key_id: impl Into<String>,
        _secret_access_key: impl Into<String>,
        _session_token: Option<String>,
    ) -> Self {
        Self {}
    }
}
