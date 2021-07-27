use std::{future::Future, net::SocketAddr};
use tokio::task::JoinHandle;

mod net;
mod rand;
mod task;
mod time;

pub struct Runtime {
    rand: rand::RandomHandle,
    time: time::TimeHandle,
    executor: task::Executor,
    net: net::NetworkRuntime,
}

impl Runtime {
    pub fn new() -> std::io::Result<Self> {
        Self::new_with_seed(0)
    }

    pub fn new_with_seed(seed: u64) -> std::io::Result<Self> {
        let rand = rand::RandomHandle::new_with_seed(seed);
        let time = time::TimeHandle::new();
        let executor = task::Executor::new()?;
        let net = net::NetworkRuntime::new(rand.clone(), time.clone(), executor.handle());
        Ok(Runtime {
            rand,
            time,
            executor,
            net,
        })
    }

    pub fn handle(&self, addr: SocketAddr) -> Handle {
        Handle {
            rand: self.rand.clone(),
            time: self.time.clone(),
            task: self.executor.handle(),
            net: self.net.handle(addr),
        }
    }

    pub fn block_on<F: Future>(&self, future: F) -> F::Output {
        self.executor.block_on(future)
    }
}

pub struct Handle {
    rand: rand::RandomHandle,
    time: time::TimeHandle,
    task: task::TaskHandle,
    net: net::NetworkHandle,
}

impl Handle {
    pub fn spawn<F>(&self, future: F) -> JoinHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        self.task.spawn(future)
    }

    pub fn net(&self) -> &net::NetworkHandle {
        &self.net
    }
}

#[cfg(test)]
fn init_logger() {
    use std::sync::Once;
    static LOGGER: Once = Once::new();
    LOGGER.call_once(|| env_logger::init());
}
