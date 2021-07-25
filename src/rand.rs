use rand::{rngs, Rng};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone)]
pub struct RandomHandle {
    rng: Arc<Mutex<rngs::SmallRng>>,
}

impl RandomHandle {
    pub fn new_with_seed(seed: u64) -> Self {
        RandomHandle {
            rng: Arc::new(Mutex::new(rand::SeedableRng::seed_from_u64(seed))),
        }
    }

    pub fn should_fault(&self, probability: f64) -> bool {
        let mut lock = self.rng.lock().unwrap();
        lock.gen_bool(probability)
    }
}
