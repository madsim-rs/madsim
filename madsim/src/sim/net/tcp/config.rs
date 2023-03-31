use std::hash::{Hash, Hasher};

use serde::{Deserialize, Serialize};

/// tcp configurations.
#[cfg_attr(docsrs, doc(cfg(madsim)))]
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone, Default)]
pub struct TcpConfig {}

#[allow(clippy::derived_hash_with_manual_eq)]
impl Hash for TcpConfig {
    fn hash<H: Hasher>(&self, _state: &mut H) {}
}
