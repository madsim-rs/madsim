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

    // pub fn access_key_id(&self) -> &str {
    //     &self.0.access_key_id
    // }

    // pub fn secret_access_key(&self) -> &str {
    //     &self.0.secret_access_key
    // }

    // pub fn expiry(&self) -> Option<SystemTime> {
    //     self.0.expires_after
    // }

    // pub fn expiry_mut(&mut self) -> &mut Option<SystemTime> {
    //     &mut Arc::make_mut(&mut self.0).expires_after
    // }

    // pub fn session_token(&self) -> Option<&str> {
    //     self.0.session_token.as_deref()
    // }
}
