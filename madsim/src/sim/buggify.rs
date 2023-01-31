//! Buggify allows you to cooperate with the simulator to inject failures.
//!
//! Learn more: <https://transactional.blog/simulation/buggify>

/// Returns true with a probability of 25% if buggify is enabled.
pub fn buggify() -> bool {
    crate::rand::thread_rng().buggify()
}

/// Buggify with given probability.
pub fn buggify_with_prob(probability: f64) -> bool {
    crate::rand::thread_rng().buggify_with_prob(probability)
}

/// Enable buggify.
pub fn enable() {
    crate::rand::thread_rng().enable_buggify()
}

/// Disable buggify.
pub fn disable() {
    crate::rand::thread_rng().disable_buggify()
}

/// Returns if buggify is enabled.
pub fn is_enabled() -> bool {
    crate::rand::thread_rng().is_buggify_enabled()
}
