use rand::{rngs, RngCore};
pub use rand::{seq, Rng};
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

    pub fn with<T>(&self, f: impl FnOnce(&mut rngs::SmallRng) -> T) -> T {
        let mut lock = self.rng.lock().unwrap();
        f(&mut *lock)
    }
}

pub fn rng() -> RandomHandle {
    crate::context::rand_handle()
}

impl RngCore for RandomHandle {
    fn next_u32(&mut self) -> u32 {
        self.rng.lock().unwrap().next_u32()
    }

    fn next_u64(&mut self) -> u64 {
        self.rng.lock().unwrap().next_u64()
    }

    fn fill_bytes(&mut self, dest: &mut [u8]) {
        self.rng.lock().unwrap().fill_bytes(dest);
    }

    fn try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), rand::Error> {
        self.rng.lock().unwrap().try_fill_bytes(dest)
    }
}
