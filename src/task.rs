use std::{future::Future, io, time::Duration};
use tokio::{
    runtime::{Builder, Handle, Runtime},
    task::JoinHandle,
};

pub struct Executor {
    runtime: Runtime,
    start_time: tokio::time::Instant,
}

impl Executor {
    pub fn new() -> io::Result<Self> {
        let runtime = Builder::new_current_thread()
            .enable_time()
            .start_paused(true)
            .build()?;
        let start_time = runtime.block_on(async { tokio::time::Instant::now() });
        Ok(Executor {
            runtime,
            start_time,
        })
    }

    pub fn handle(&self) -> TaskHandle {
        TaskHandle {
            handle: self.runtime.handle().clone(),
        }
    }

    pub fn block_on<F: Future>(&self, future: F) -> F::Output {
        self.runtime.block_on(future)
    }

    pub fn current_time(&self) -> Duration {
        self.runtime.block_on(async { self.start_time.elapsed() })
    }
}

#[derive(Debug, Clone)]
pub struct TaskHandle {
    handle: Handle,
}

impl TaskHandle {
    pub fn spawn<F>(&self, future: F) -> JoinHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        self.handle.spawn(future)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn time() {
        let executor = Executor::new().unwrap();
        assert_eq!(executor.current_time(), Duration::default());

        executor.block_on(tokio::time::advance(Duration::from_secs(1)));
        assert_eq!(executor.current_time(), Duration::from_secs(1));
    }
}
