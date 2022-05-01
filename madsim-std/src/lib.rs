use std::{
    collections::HashMap,
    future::Future,
    io,
    net::{SocketAddr, ToSocketAddrs},
    sync::{mpsc, Arc, Mutex},
    thread::JoinHandle,
    time::Duration,
};

use tokio::sync::mpsc::UnboundedSender;

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
    local: tokio::task::LocalSet,
    handle: Handle,
    local_handle: LocalHandle,
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
        let local = tokio::task::LocalSet::new();
        let handle = Handle {
            locals: Default::default(),
            net: net::NetHandle::new(),
            fs: fs::FsHandle::new(),
        };
        let local_handle = LocalHandle {
            handle: rt.handle().clone(),
            net: handle.net.create_host(&rt, &local, "127.0.0.1:0").unwrap(),
            term: None,
        };
        Runtime {
            rt,
            local,
            handle,
            local_handle,
        }
    }

    /// Return a handle to the runtime.
    ///
    /// The returned handle can be used by the supervisor (future in [block_on])
    /// to control the whole system. For example, kill a host or disconnect the
    /// network.
    ///
    /// [block_on]: Runtime::block_on
    pub fn handle(&self) -> &Handle {
        &self.handle
    }

    /// Create a host which will be bound to the specified address.
    ///
    /// The returned handle can be used to spawn tasks that run on this host.
    pub fn create_host(&self, addr: impl ToSocketAddrs) -> HostBuilder<'_> {
        self.handle.create_host(addr)
    }

    /// Run a future to completion on the runtime. This is the runtime’s entry point.
    pub fn block_on<F: Future>(&self, future: F) -> F::Output {
        let _guard = crate::context::enter(self.handle.clone());
        let _local_guard = crate::context::enter_local(self.local_handle.clone());
        self.local.block_on(&self.rt, future)
    }

    /// Set a time limit of the execution.
    pub fn set_time_limit(&mut self, _limit: Duration) {
        todo!()
    }

    /// Dummy. Do NOT call.
    pub fn enable_deterministic_check(&self, _log: Option<()>) {
        panic!("madsim-std does not support deterministic check");
    }

    /// Dummy. Do NOT call.
    pub fn take_rand_log(self) -> Option<()> {
        None
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
    pub fn create_host(&self, addr: impl ToSocketAddrs) -> HostBuilder<'_> {
        let addr = addr.to_socket_addrs().unwrap().next().unwrap();
        HostBuilder::new(self, addr)
    }

    /// Return a handle of the specified host.
    pub fn get_host(&self, addr: SocketAddr) -> Option<LocalHandle> {
        self.locals.lock().unwrap().get(&addr).cloned()
    }
}

/// Builds a host with custom configurations.
pub struct HostBuilder<'a> {
    handle: &'a Handle,
    addr: SocketAddr,
    name: Option<String>,
    init: Option<Arc<dyn Fn()>>,
}

impl<'a> HostBuilder<'a> {
    fn new(handle: &'a Handle, addr: SocketAddr) -> Self {
        HostBuilder {
            handle,
            addr,
            name: None,
            init: None,
        }
    }

    /// Names the host.
    ///
    /// The default name is socket address.
    pub fn name(mut self, name: String) -> Self {
        self.name = Some(name);
        self
    }

    /// Set the initial task for the host.
    ///
    /// This task will be automatically respawned after crash.
    pub fn init<F>(mut self, future: impl Fn() -> F + 'static) -> Self
    where
        F: Future + 'static,
    {
        self.init = Some(Arc::new(move || {
            task::spawn_local(future()).detach();
        }));
        self
    }

    /// Build a host.
    pub fn build(self) -> io::Result<LocalHandle> {
        let handle = LocalHandle::new(self.handle, self.addr)?;
        self.handle
            .locals
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
    term: Option<TermHandle>,
}

#[derive(Clone)]
struct TermHandle {
    tx: UnboundedSender<()>,
    join: Arc<Mutex<Option<JoinHandle<()>>>>,
}

impl LocalHandle {
    /// SIGTERM, can only call once.
    pub async fn terminate(&mut self) {
        if let Some(term) = &mut self.term {
            term.tx.send(()).unwrap();
            term.join.lock().unwrap().take().unwrap().join().unwrap();
        };
    }

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

    fn new(handle: &Handle, addr: impl ToSocketAddrs) -> io::Result<Self> {
        let addr = addr.to_socket_addrs()?.next().unwrap();
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        // create a channel to receive the local_handle from the new thread
        let (sender, recver) = mpsc::channel();
        let handle = handle.clone();
        let (kill_tx, mut kill_rx) = tokio::sync::mpsc::unbounded_channel();
        let join = std::thread::spawn(move || {
            let local = tokio::task::LocalSet::new();
            let local_handle = LocalHandle {
                handle: rt.handle().clone(),
                net: handle.net.create_host(&rt, &local, addr).unwrap(),
                term: Some(TermHandle {
                    tx: kill_tx,
                    join: Arc::new(Mutex::new(None)),
                }),
            };
            sender.send(local_handle.clone()).ok().unwrap();

            let _guard = crate::context::enter_local(local_handle);
            local.block_on(&rt, async move {
                kill_rx.recv().await;
            });
        });
        let mut handle = recver.recv().unwrap();
        if let Some(term) = &mut handle.term {
            term.join = Arc::new(Mutex::new(Some(join)));
        }
        Ok(handle)
    }
}

#[cfg(feature = "logger")]
fn init_logger() {
    use std::sync::Once;
    static LOGGER_INIT: Once = Once::new();
    LOGGER_INIT.call_once(env_logger::init);
}
