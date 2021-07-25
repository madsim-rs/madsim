use std::{future::Future, io};
use tokio::runtime::{Builder, Handle, Runtime};

pub struct Executor {
    runtime: Runtime,
}

impl Executor {
    pub fn new() -> io::Result<Self> {
        Ok(Executor {
            runtime: Builder::new_current_thread()
                .enable_time()
                .start_paused(true)
                .build()?,
        })
    }

    pub fn handle(&self) -> Spawner {
        Spawner {
            handle: self.runtime.handle().clone(),
        }
    }

    pub fn block_on<F: Future>(&self, future: F) -> F::Output {
        self.runtime.block_on(future)
    }
}

#[derive(Debug, Clone)]
pub struct Spawner {
    handle: Handle,
}

impl Spawner {
    pub fn spawn<F>(&self, future: F)
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        self.handle.spawn(future);
    }
}
