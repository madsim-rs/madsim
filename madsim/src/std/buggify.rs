//! Buggify allows you to cooperate with the simulator to inject failures.
//!
//! It always returns false when not running in simulation mode.

/// Returns true with a probability of 25% if buggify is enabled.
#[inline(always)]
pub fn buggify() -> bool {
    false
}

/// Buggify with given probability.
#[inline(always)]
pub fn buggify_with_prob(_probability: f64) -> bool {
    false
}

/// Enable buggify.
#[inline(always)]
pub fn enable() {}

/// Disable buggify.
#[inline(always)]
pub fn disable() {}

/// Returns if buggify is enabled.
#[inline(always)]
pub fn is_enabled() -> bool {
    false
}
