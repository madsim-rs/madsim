use std::hash::{Hash, Hasher};

use serde::{Deserialize, Serialize};

/// tcp configurations.
#[cfg_attr(docsrs, doc(cfg(madsim)))]
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone, Default)]
pub struct TcpConfig {}

#[allow(clippy::derive_hash_xor_eq)]
impl Hash for TcpConfig {
    fn hash<H: Hasher>(&self, _state: &mut H) {}
}
