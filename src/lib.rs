use std::future::Future;

mod executor;
mod net;
mod rand;
mod time;

pub struct Runtime {
    rand: rand::RandomHandle,
    time: time::TimeHandle,
    executor: executor::Executor,
    net: net::NetworkRuntime,
}

impl Runtime {
    pub fn new() -> std::io::Result<Self> {
        Self::new_with_seed(0)
    }

    pub fn new_with_seed(seed: u64) -> std::io::Result<Self> {
        let rand = rand::RandomHandle::new_with_seed(seed);
        let time = time::TimeHandle::new();
        let executor = executor::Executor::new()?;
        let net = net::NetworkRuntime::new(rand.clone(), time.clone(), executor.handle());
        Ok(Runtime {
            rand,
            time,
            executor,
            net,
        })
    }

    pub fn block_on<F: Future>(&self, future: F) -> F::Output {
        self.executor.block_on(future)
    }
}
