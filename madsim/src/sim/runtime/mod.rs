//! The madsim runtime.

use super::*;
use crate::task::{JoinHandle, NodeId, ToNodeId};
use spin::Mutex;
use std::{
    any::{Any, TypeId},
    collections::HashMap,
    future::Future,
    net::IpAddr,
    sync::Arc,
    time::Duration,
};

mod builder;
pub(crate) mod context;
mod metrics;

pub use self::builder::Builder;
pub use self::metrics::RuntimeMetrics;

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
    rand: rand::GlobalRng,
    task: task::Executor,
    handle: Handle,
}

impl Default for Runtime {
    fn default() -> Self {
        Self::new()
    }
}

impl Runtime {
    /// Create a new runtime instance with default seed and config.
    pub fn new() -> Self {
        Self::with_seed_and_config(0, Config::default())
    }

    /// Create a new runtime instance with given seed and config.
    pub fn with_seed_and_config(seed: u64, config: Config) -> Self {
        let rand = rand::GlobalRng::new_with_seed(seed);
        let sims = Arc::new(Mutex::new(HashMap::new()));
        let task = task::Executor::new(rand.clone(), sims.clone());
        let handle = Handle {
            rand: rand.clone(),
            time: task.time_handle().clone(),
            task: task.handle().clone(),
            sims,
            config,
        };
        let rt = Runtime { rand, task, handle };
        rt.add_simulator::<fs::FsSim>();
        rt.add_simulator::<net::NetSim>();
        rt
    }

    /// Register a simulator.
    pub fn add_simulator<S: plugin::Simulator>(&self) {
        let mut sims = self.handle.sims.lock();
        let sim = Arc::new(S::new1(
            &self.handle.rand,
            &self.handle.time,
            &self.handle.task.get_node(NodeId::zero()).unwrap(),
            &self.handle.config,
        ));
        // create node for supervisor
        sim.create_node(NodeId::zero());
        sims.insert(TypeId::of::<S>(), sim);
    }

    /// Return a handle to the runtime.
    ///
    /// The returned handle can be used by the supervisor (future in [block_on])
    /// to control the whole system. For example, kill a node or disconnect the
    /// network.
    ///
    /// [block_on]: Runtime::block_on
    pub fn handle(&self) -> &Handle {
        &self.handle
    }

    /// Create a node.
    ///
    /// The returned handle can be used to spawn tasks that run on this node.
    pub fn create_node(&self) -> NodeBuilder<'_> {
        self.handle.create_node()
    }

    /// Run a future to completion on the runtime. This is the runtime’s entry
    /// point.
    ///
    /// This runs the given future on the current thread until it is complete.
    ///
    /// # Example
    ///
    /// ```
    /// use madsim::runtime::Runtime;
    ///
    /// let rt = Runtime::new();
    /// let ret = rt.block_on(async { 1 });
    /// assert_eq!(ret, 1);
    /// ```
    ///
    /// Unlike usual async runtime, when there is no runnable task, it will
    /// panic instead of blocking.
    ///
    /// ```should_panic
    /// use madsim::runtime::Runtime;
    /// use std::future::pending;
    ///
    /// Runtime::new().block_on(pending::<()>());
    /// ```
    pub fn block_on<F: Future>(&self, future: F) -> F::Output {
        let _guard = crate::context::enter(self.handle.clone());
        self.task.block_on(future)
    }

    /// Set a time limit of the execution.
    ///
    /// The runtime will panic when time limit exceeded.
    ///
    /// # Example
    ///
    /// ```should_panic
    /// use madsim::{runtime::Runtime, time::{sleep, Duration}};
    ///
    /// let mut rt = Runtime::new();
    /// rt.set_time_limit(Duration::from_secs(1));
    ///
    /// rt.block_on(async {
    ///     sleep(Duration::from_secs(2)).await;
    /// });
    /// ```
    pub fn set_time_limit(&mut self, limit: Duration) {
        self.task.set_time_limit(limit);
    }

    /// Check determinism of the future.
    ///
    /// # Example
    ///
    /// ```should_panic
    /// use madsim::{Config, runtime::Runtime, time::{Duration, sleep}};
    /// use std::io::Read;
    /// use rand::Rng;
    ///
    /// Runtime::check_determinism(0, Config::default(), || async {
    ///     // read a real random number from OS
    ///     let mut file = std::fs::File::open("/dev/urandom").unwrap();
    ///     let mut buf = [0u8; 8];
    ///     file.read_exact(&mut buf).unwrap();
    ///
    ///     sleep(Duration::from_nanos(u64::from_ne_bytes(buf))).await;
    /// });
    /// ```
    pub fn check_determinism<F>(seed: u64, config: Config, f: fn() -> F) -> F::Output
    where
        F: Future + 'static,
        F::Output: Send,
    {
        let config0 = config.clone();
        let log = std::thread::spawn(move || {
            let rt = Runtime::with_seed_and_config(seed, config0);
            rt.rand.enable_log();
            rt.block_on(f());
            rt.rand.take_log().unwrap()
        })
        .join()
        .map_err(|e| panic_with_info(seed, e))
        .unwrap();

        std::thread::spawn(move || {
            let rt = Runtime::with_seed_and_config(seed, config);
            rt.rand.enable_check(log);
            rt.block_on(f())
        })
        .join()
        .map_err(|e| panic_with_info(seed, e))
        .unwrap()
    }
}

fn panic_with_info(seed: u64, payload: Box<dyn Any + Send>) -> ! {
    eprintln!(
        "note: run with `MADSIM_TEST_SEED={seed}` environment variable to reproduce this error"
    );
    std::panic::resume_unwind(payload);
}

/// Supervisor handle to the runtime.
#[derive(Clone)]
pub struct Handle {
    pub(crate) rand: rand::GlobalRng,
    pub(crate) time: time::TimeHandle,
    pub(crate) task: task::TaskHandle,
    pub(crate) sims: Arc<Simulators>,

    pub(crate) config: Config,
}

/// A collection of simulators.
pub(crate) type Simulators = Mutex<HashMap<TypeId, Arc<dyn plugin::Simulator>>>;

/// `TryCurrentError` indicates there is no runtime has been started
#[derive(Debug)]
pub struct TryCurrentError;

impl Handle {
    /// Returns a [`Handle`] view over the currently running [`Runtime`].
    ///
    /// ## Panic
    ///
    /// This will panic if called outside the context of a Madsim runtime.
    ///
    /// ```should_panic
    /// let handle = madsim::runtime::Handle::current();
    /// ```
    pub fn current() -> Self {
        context::current(|h| h.clone())
    }

    /// Returns a [`Handle`] view over the currently running [`Runtime`]
    ///
    /// Returns an error if no Runtime has been started
    ///
    /// Contrary to `current`, this never panics
    pub fn try_current() -> Result<Self, TryCurrentError> {
        context::try_current(|h| h.clone()).ok_or(TryCurrentError)
    }

    /// spawn a task
    pub fn spawn<F>(&self, future: F) -> JoinHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        self.create_node()
            .name("spawn a task")
            .build()
            .spawn(future)
    }

    /// spawn a blocking task
    #[deprecated(
        since = "0.3.0",
        note = "blocking function is not allowed in simulation"
    )]
    pub fn spawn_blocking<F, R>(&self, _f: F) -> JoinHandle<R>
    where
        F: FnOnce() -> R + Send + 'static,
        R: Send + 'static,
    {
        unimplemented!()
    }

    /// Returns the random seed of the current runtime.
    ///
    /// ```
    /// use madsim::{Config, runtime::Runtime};
    ///
    /// let rt = Runtime::with_seed_and_config(2333, Config::default());
    /// assert_eq!(rt.handle().seed(), 2333);
    /// ```
    pub fn seed(&self) -> u64 {
        self.rand.seed()
    }

    /// Kill a node.
    ///
    /// - All tasks spawned on this node will be killed immediately.
    /// - All data that has not been flushed to the disk will be lost.
    pub fn kill(&self, id: impl ToNodeId) {
        self.task.kill(&id);
    }

    /// Restart a node。
    pub fn restart(&self, id: impl ToNodeId) {
        self.task.restart(&id);
    }

    /// Pause the execution of a node.
    pub fn pause(&self, id: impl ToNodeId) {
        self.task.pause(id);
    }

    /// Resume the execution of a node.
    pub fn resume(&self, id: impl ToNodeId) {
        self.task.resume(id);
    }

    /// Send a Ctrl+C signal to the node.
    pub fn send_ctrl_c(&self, id: impl ToNodeId) {
        self.task.send_ctrl_c(id);
    }

    /// Returns whether the node is killed or exited.
    pub fn is_exit(&self, id: impl ToNodeId) -> bool {
        self.task.is_exit(id)
    }

    /// Create a node which will be bound to the specified address.
    pub fn create_node(&self) -> NodeBuilder<'_> {
        NodeBuilder::new(self)
    }

    /// Return a handle of the specified node.
    pub fn get_node(&self, id: impl ToNodeId) -> Option<NodeHandle> {
        self.task.get_node(id).map(|task| NodeHandle { task })
    }

    /// Returns a view that lets you get information about how the runtime is
    /// performing.
    pub fn metrics(&self) -> RuntimeMetrics {
        RuntimeMetrics {
            task: self.task.clone(),
        }
    }
}

/// Builds a node with custom configurations.
pub struct NodeBuilder<'a> {
    handle: &'a Handle,
    pub(crate) name: Option<String>,
    pub(crate) ip: Option<IpAddr>,
    pub(crate) cores: Option<usize>,
    pub(crate) init: Option<task::InitFn>,
    pub(crate) restart_on_panic: bool,
    pub(crate) restart_on_panic_matching: Vec<String>,
}

impl<'a> NodeBuilder<'a> {
    fn new(handle: &'a Handle) -> Self {
        NodeBuilder {
            handle,
            name: None,
            ip: None,
            cores: None,
            init: None,
            restart_on_panic: false,
            restart_on_panic_matching: vec![],
        }
    }

    /// Names the node.
    ///
    /// The default name is node ID.
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Set the initial task for the node.
    ///
    /// This task will be respawned when calling `restart`.
    pub fn init<F>(mut self, new_task: impl Fn() -> F + Send + Sync + 'static) -> Self
    where
        F: Future + 'static,
    {
        self.init = Some(Arc::new(move |handle| {
            let future = new_task();
            let h = handle.clone();
            handle.spawn_local(async move {
                future.await;
                h.exit();
            });
        }));
        self
    }

    /// Automatically restart the node when it panics.
    ///
    /// By default a panic will terminate the simulation.
    pub fn restart_on_panic(mut self) -> Self {
        self.restart_on_panic = true;
        self
    }

    /// Automatically restart the node when it panics with a message containing
    /// the given string.
    pub fn restart_on_panic_matching(mut self, msg: impl Into<String>) -> Self {
        self.restart_on_panic_matching.push(msg.into());
        self
    }

    /// Set one IP address of the node.
    pub fn ip(mut self, ip: IpAddr) -> Self {
        self.ip = Some(ip);
        self
    }

    /// Set the number of CPU cores of the node.
    ///
    /// This will be the return value of [`std::thread::available_parallelism`].
    pub fn cores(mut self, cores: usize) -> Self {
        assert_ne!(cores, 0, "cores must be greater than 0");
        self.cores = Some(cores);
        self
    }

    /// Build a node.
    pub fn build(self) -> NodeHandle {
        let task = self.handle.task.create_node(&self);
        let sims = self.handle.sims.lock();
        let values = sims.values();
        for sim in values {
            sim.create_node(task.node_id());
            if let Some(ip) = self.ip {
                if let Some(net) = sim.downcast_ref::<net::NetSim>() {
                    net.set_ip(task.node_id(), ip)
                }
            }
        }
        NodeHandle { task }
    }
}

/// Handle to a node.
#[derive(Clone)]
pub struct NodeHandle {
    task: task::Spawner,
}

impl NodeHandle {
    /// Returns the node ID.
    pub fn id(&self) -> NodeId {
        self.task.node_id()
    }

    /// Spawn a future onto the runtime.
    #[track_caller]
    pub fn spawn<F>(&self, future: F) -> JoinHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        self.task.spawn(future)
    }
}

/// Initialize logger.
pub fn init_logger() {
    use std::sync::Once;
    static LOGGER_INIT: Once = Once::new();
    LOGGER_INIT.call_once(tracing_subscriber::fmt::init);
}
