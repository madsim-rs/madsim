use std::{future::Future, net::SocketAddr, time::Duration};

pub mod fs;
pub mod net;
pub mod rand;
pub mod task;
pub mod time;

/// The madsim runtime.
///
/// The runtime provides basic components for deterministic simulation,
/// including a [random number generator], [timer], [task scheduler], and
/// simulated [network] and [file system].
///
/// [random number generator]: crate::rand
/// [timer]: crate::time
/// [task scheduler]: crate::task
/// [network]: crate::net
/// [file system]: crate::fs
pub struct Runtime {
    rt: tokio::runtime::Runtime,
}

impl Default for Runtime {
    fn default() -> Self {
        Self::new()
    }
}

impl Runtime {
    /// Create a new runtime instance.
    pub fn new() -> Self {
        Self::new_with_seed(0)
    }

    /// Create a new runtime instance with given seed.
    pub fn new_with_seed(_seed: u64) -> Self {
        #[cfg(feature = "logger")]
        crate::init_logger();

        Runtime {
            rt: tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap(),
        }
    }

    /// Return a handle to the runtime.
    ///
    /// The returned handle can be used by the supervisor (future in [block_on])
    /// to control the whole system. For example, kill a host or disconnect the
    /// network.
    ///
    /// [block_on]: Runtime::block_on
    pub fn handle(&self) -> Handle {
        Handle {
            handle: self.rt.handle().clone(),
            net: todo!(),
            fs: todo!(),
        }
    }

    /// Return a handle of the specified host.
    ///
    /// The returned handle can be used to spawn tasks that run on this host.
    pub fn local_handle(&self, addr: SocketAddr) -> LocalHandle {
        LocalHandle {
            handle: self.rt.handle().clone(),
        }
    }

    /// Run a future to completion on the runtime. This is the runtime’s entry point.
    pub fn block_on<F: Future>(&self, future: F) -> F::Output {
        self.rt.block_on(future)
    }
}

/// Supervisor handle to the runtime.
#[derive(Clone)]
#[allow(missing_docs)]
pub struct Handle {
    handle: tokio::runtime::Handle,
    pub net: net::NetHandle,
    pub fs: fs::FsHandle,
}

impl Handle {
    /// Returns a [`Handle`] view over the currently running [`Runtime`].
    ///
    /// ## Panic
    ///
    /// This will panic if called outside the context of a Madsim runtime.
    ///
    /// ```should_panic
    /// let handle = madsim::Handle::current();
    /// ```
    pub fn current() -> Self {
        Handle {
            handle: tokio::runtime::Handle::current(),
            net: todo!(),
            fs: todo!(),
        }
    }

    /// Kill a host.
    pub fn kill(&self, _addr: SocketAddr) {
        todo!()
    }

    /// Return a handle of the specified host.
    pub fn local_handle(&self, _addr: SocketAddr) -> LocalHandle {
        LocalHandle {
            handle: self.handle.clone(),
        }
    }
}

/// Local host handle to the runtime.
#[derive(Clone)]
pub struct LocalHandle {
    handle: tokio::runtime::Handle,
}

impl LocalHandle {
    /// Spawn a future onto the runtime.
    pub fn spawn<F>(&self, future: F) -> task::Task<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        task::Task(self.handle.spawn(future))
    }
}

#[cfg(feature = "logger")]
fn init_logger() {
    use std::sync::Once;
    static LOGGER_INIT: Once = Once::new();
    LOGGER_INIT.call_once(|| env_logger::init());
}
