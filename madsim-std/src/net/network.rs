use std::{ops::Range, time::Duration};

/// Network configurations.
#[derive(Debug, Default)]
#[allow(missing_docs)]
pub struct Config {
    pub packet_loss_rate: f64,
    pub send_latency: Range<Duration>,
}

/// Network statistics.
#[derive(Debug, Default, Clone)]
pub struct Stat {
    /// Total number of messages.
    pub msg_count: u64,
}
