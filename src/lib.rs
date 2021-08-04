use std::{future::Future, net::SocketAddr};

mod context;
pub mod fs;
pub mod net;
pub mod rand;
pub mod task;
pub mod time;

pub struct Runtime {
    rand: rand::RandomHandle,
    task: task::Executor,
    net: net::NetworkRuntime,
    fs: fs::FileSystemRuntime,
}

impl Runtime {
    pub fn new() -> Self {
        Self::new_with_seed(0)
    }

    pub fn new_with_seed(seed: u64) -> Self {
        #[cfg(test)]
        crate::init_logger();

        let rand = rand::RandomHandle::new_with_seed(seed);
        let task = task::Executor::new();
        let net = net::NetworkRuntime::new(rand.clone(), task.time_handle().clone());
        let fs = fs::FileSystemRuntime::new(rand.clone(), task.time_handle().clone());
        Runtime {
            rand,
            task,
            net,
            fs,
        }
    }

    pub fn handle(&self) -> Handle {
        Handle {
            rand: self.rand.clone(),
            time: self.task.time_handle().clone(),
            task: self.task.handle().clone(),
            net: self.net.handle().clone(),
            fs: self.fs.handle().clone(),
        }
    }

    pub fn local_handle(&self, addr: SocketAddr) -> LocalHandle {
        LocalHandle {
            rand: self.rand.clone(),
            time: self.task.time_handle().clone(),
            task: self.task.handle().local_handle(addr),
            net: self.net.handle().local_handle(addr),
            fs: self.fs.handle().local_handle(addr),
        }
    }

    pub fn block_on<F: Future>(&self, future: F) -> F::Output {
        let _guard = crate::context::enter(self.handle());
        self.task.block_on(future)
    }
}

#[derive(Clone)]
pub struct Handle {
    pub rand: rand::RandomHandle,
    pub time: time::TimeHandle,
    pub task: task::TaskHandle,
    pub net: net::NetworkHandle,
    pub fs: fs::FileSystemHandle,
}

impl Handle {
    pub fn current() -> Self {
        context::current().expect("no madsim context")
    }
}

#[derive(Clone)]
pub struct LocalHandle {
    pub rand: rand::RandomHandle,
    pub time: time::TimeHandle,
    pub task: task::TaskLocalHandle,
    pub net: net::NetworkLocalHandle,
    pub fs: fs::FileSystemLocalHandle,
}

impl LocalHandle {
    pub fn spawn<F>(&self, future: F) -> async_task::Task<F::Output>
    where
        F: Future + 'static,
        F::Output: 'static,
    {
        self.task.spawn(future)
    }
}

#[cfg(test)]
fn init_logger() {
    use std::sync::Once;
    static LOGGER: Once = Once::new();
    LOGGER.call_once(|| env_logger::init());
}
