use std::{
    collections::HashMap,
    future::Future,
    io,
    net::{SocketAddr, ToSocketAddrs},
    sync::{Arc, Mutex},
    time::Duration,
};

mod context;
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
    handle: Handle,
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

        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        Runtime {
            handle: Handle {
                locals: Default::default(),
                net: net::NetHandle::new(),
                fs: fs::FsHandle::new(),
            },
            rt,
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
        self.handle.clone()
    }

    /// Create a host which will be bound to the specified address.
    ///
    /// The returned handle can be used to spawn tasks that run on this host.
    pub fn create_host(&self, addr: impl ToSocketAddrs) -> io::Result<LocalHandle> {
        self.handle.create_host(addr)
    }

    /// Run a future to completion on the runtime. This is the runtime’s entry point.
    pub fn block_on<F: Future>(&self, future: F) -> F::Output {
        let _guard = crate::context::enter(self.handle());
        self.rt.block_on(future)
    }
}

/// Supervisor handle to the runtime.
#[derive(Clone)]
#[allow(missing_docs)]
pub struct Handle {
    locals: Arc<Mutex<HashMap<SocketAddr, LocalHandle>>>,
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
    /// let handle = madsim_std::Handle::current();
    /// ```
    pub fn current() -> Self {
        crate::context::current()
    }

    /// Kill a host.
    pub fn kill(&self, _addr: SocketAddr) {
        todo!()
    }

    /// Create a host which will be bound to the specified address.
    pub fn create_host(&self, addr: impl ToSocketAddrs) -> io::Result<LocalHandle> {
        let handle = LocalHandle::new(addr)?;
        self.locals
            .lock()
            .unwrap()
            .insert(handle.local_addr(), handle.clone());
        Ok(handle)
    }
}

/// Local host handle to the runtime.
#[derive(Clone)]
pub struct LocalHandle {
    handle: tokio::runtime::Handle,
    net: net::NetLocalHandle,
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

    /// Returns the local socket address.
    pub fn local_addr(&self) -> SocketAddr {
        self.net.local_addr()
    }

    fn new(addr: impl ToSocketAddrs) -> io::Result<Self> {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        let handle = LocalHandle {
            handle: rt.handle().clone(),
            net: net::NetLocalHandle::new(rt.handle(), addr)?,
        };
        let handle0 = handle.clone();
        std::thread::spawn(move || {
            let _guard = crate::context::enter_local(handle0);
            rt.block_on(futures::future::pending::<()>());
        });
        Ok(handle)
    }
}

#[cfg(feature = "logger")]
fn init_logger() {
    use std::sync::Once;
    static LOGGER_INIT: Once = Once::new();
    LOGGER_INIT.call_once(|| env_logger::init());
}
