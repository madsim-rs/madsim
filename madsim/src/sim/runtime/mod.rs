//! The madsim runtime.

use super::*;
use crate::task::{JoinHandle, NodeId};
use std::{
    any::{Any, TypeId},
    collections::HashMap,
    future::Future,
    net::IpAddr,
    sync::{Arc, Mutex},
    time::Duration,
};

mod builder;
pub(crate) mod context;

pub use self::builder::Builder;

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
        let task = task::Executor::new(rand.clone());
        let handle = Handle {
            rand: rand.clone(),
            time: task.time_handle().clone(),
            task: task.handle().clone(),
            sims: Default::default(),
            config,
        };
        let rt = Runtime { rand, task, handle };
        rt.add_simulator::<fs::FsSim>();
        rt.add_simulator::<net::NetSim>();
        rt
    }

    /// Register a simulator.
    pub fn add_simulator<S: plugin::Simulator>(&self) {
        let mut sims = self.handle.sims.lock().unwrap();
        let sim = Arc::new(S::new(
            &self.handle.rand,
            &self.handle.time,
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

    /// Run a future to completion on the runtime. This is the runtime’s entry point.
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
    /// use futures::future::pending;
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
    ///     let mut buf = [0u8];
    ///     file.read_exact(&mut buf).unwrap();
    ///
    ///     sleep(Duration::from_nanos(buf[0] as _)).await;
    /// });
    /// ```
    pub fn check_determinism<F>(seed: u64, config: Config, f: fn() -> F) -> F::Output
    where
        F: Future + 'static,
        F::Output: Send,
    {
        let hash = config.hash();
        let config0 = config.clone();
        let log = std::thread::spawn(move || {
            let rt = Runtime::with_seed_and_config(seed, config0);
            rt.rand.enable_log();
            rt.block_on(f());
            rt.rand.take_log().unwrap()
        })
        .join()
        .map_err(|e| panic_with_info(seed, hash, e))
        .unwrap();

        std::thread::spawn(move || {
            let rt = Runtime::with_seed_and_config(seed, config);
            rt.rand.enable_check(log);
            rt.block_on(f())
        })
        .join()
        .map_err(|e| panic_with_info(seed, hash, e))
        .unwrap()
    }
}

fn panic_with_info(seed: u64, hash: u64, payload: Box<dyn Any + Send>) -> ! {
    eprintln!(
        "note: run with `MADSIM_TEST_SEED={}` environment variable to reproduce this error",
        seed
    );
    eprintln!("      and make sure `MADSIM_CONFIG_HASH={:016X}`", hash);
    std::panic::resume_unwind(payload);
}

/// Supervisor handle to the runtime.
#[derive(Clone)]
pub struct Handle {
    pub(crate) rand: rand::GlobalRng,
    pub(crate) time: time::TimeHandle,
    pub(crate) task: task::TaskHandle,
    pub(crate) sims: Arc<Mutex<HashMap<TypeId, Arc<dyn plugin::Simulator>>>>,
    pub(crate) config: Config,
}

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
    pub fn kill(&self, id: NodeId) {
        self.task.kill(id);
        let sims = self.sims.lock().unwrap();
        let values = sims.values();
        for sim in values {
            sim.reset_node(id);
        }
    }

    /// Restart a node。
    pub fn restart(&self, id: NodeId) {
        self.task.restart(id);
        let sims = self.sims.lock().unwrap();
        let values = sims.values();
        for sim in values {
            sim.reset_node(id);
        }
    }

    /// Pause the execution of a node.
    pub fn pause(&self, id: NodeId) {
        self.task.pause(id);
    }

    /// Resume the execution of a node.
    pub fn resume(&self, id: NodeId) {
        self.task.resume(id);
    }

    /// Create a node which will be bound to the specified address.
    pub fn create_node(&self) -> NodeBuilder<'_> {
        NodeBuilder::new(self)
    }

    /// Return a handle of the specified node.
    pub fn get_node(&self, id: NodeId) -> Option<NodeHandle> {
        self.task.get_node(id).map(|task| NodeHandle { task })
    }
}

/// Builds a node with custom configurations.
pub struct NodeBuilder<'a> {
    handle: &'a Handle,
    name: Option<String>,
    ip: Option<IpAddr>,
    cores: Option<usize>,
    init: Option<Arc<dyn Fn(&task::TaskNodeHandle)>>,
}

impl<'a> NodeBuilder<'a> {
    fn new(handle: &'a Handle) -> Self {
        NodeBuilder {
            handle,
            name: None,
            ip: None,
            cores: None,
            init: None,
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
    /// This task will be automatically respawned after crash.
    pub fn init<F>(mut self, future: impl Fn() -> F + 'static) -> Self
    where
        F: Future + 'static,
    {
        self.init = Some(Arc::new(move |handle| {
            handle.spawn_local(future());
        }));
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
        let task = self
            .handle
            .task
            .create_node(self.name, self.init, self.cores);
        let sims = self.handle.sims.lock().unwrap();
        let values = sims.values();
        for sim in values {
            sim.create_node(task.id());
            if let Some(ip) = self.ip {
                if let Some(net) = sim.downcast_ref::<net::NetSim>() {
                    net.set_ip(task.id(), ip)
                }
            }
        }
        NodeHandle { task }
    }
}

/// Handle to a node.
#[derive(Clone)]
pub struct NodeHandle {
    task: task::TaskNodeHandle,
}

impl NodeHandle {
    /// Returns the node ID.
    pub fn id(&self) -> NodeId {
        self.task.id()
    }

    /// Spawn a future onto the runtime.
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
    use env_logger::fmt::Color;
    use std::io::Write;
    use std::sync::Once;
    static LOGGER_INIT: Once = Once::new();
    LOGGER_INIT.call_once(|| {
        let start = std::env::var("MADSIM_LOG_TIME_START")
            .ok()
            .map(|s| Duration::from_secs_f64(s.parse::<f64>().unwrap()));
        let mut builder = env_logger::Builder::from_default_env();
        builder.format(move |buf, record| {
            // filter time
            if let Some(time) = crate::time::TimeHandle::try_current() {
                if matches!(start, Some(t0) if time.elapsed() < t0) {
                    return write!(buf, "");
                }
            }
            let mut style = buf.style();
            style.set_color(Color::Black).set_intense(true);
            let mut level_style = buf.style();
            level_style.set_color(match record.level() {
                log::Level::Error => Color::Red,
                log::Level::Warn => Color::Yellow,
                log::Level::Info => Color::Green,
                log::Level::Debug => Color::Blue,
                log::Level::Trace => Color::Cyan,
            });
            write!(buf, "{}", style.value('['))?;
            if let Some(time) = crate::time::TimeHandle::try_current() {
                write!(buf, "{:.9}s", time.elapsed().as_secs_f64())?;
            }
            write!(buf, " {:>5}", level_style.value(record.level()))?;
            if let Some(task) = crate::context::try_current_task() {
                write!(buf, " {}", task.name)?;
            }
            write!(buf, " {:>10}", style.value(record.target()))?;
            writeln!(buf, "{} {}", style.value(']'), record.args())
        });
        builder.init();
    });
}
