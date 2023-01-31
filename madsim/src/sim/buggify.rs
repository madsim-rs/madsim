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

#[cfg(test)]
mod tests {
    use crate::runtime::Runtime;

    #[test]
    fn buggify() {
        let runtime = Runtime::new();
        runtime.block_on(async move {
            assert!(
                !crate::buggify::is_enabled(),
                "buggify should be disabled by default"
            );

            crate::buggify::enable();
            assert!(crate::buggify::is_enabled());

            let count = (0..1000).filter(|_| crate::buggify::buggify()).count();
            assert!((200..300).contains(&count)); // 25%

            let count = (0..1000)
                .filter(|_| crate::buggify::buggify_with_prob(0.1))
                .count();
            assert!((50..150).contains(&count)); // 10%

            crate::buggify::disable();
            assert!(!crate::buggify::is_enabled());

            for _ in 0..10 {
                assert!(!crate::buggify::buggify());
                assert!(!crate::buggify::buggify_with_prob(1.0));
            }
        });
    }
}
