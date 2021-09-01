//! Random number generator.

#[doc(no_inline)]
pub use rand::prelude::{
    CryptoRng, Distribution, IteratorRandom, Rng, RngCore, SeedableRng, SliceRandom,
};

/// Retrieve the deterministic random number generator from the current madsim context.
pub fn rng() -> impl RngCore {
    rand::thread_rng()
}
