//! A deterministic simulator for distributed systems.
//!
//! ## Features
//!
//! - `rpc`: Enables RPC through network.
//! - `logger`: Enables built-in logger.
//! - `macros`: Enables `#[madsim::main]` and `#[madsim::test]` macros.

#![deny(missing_docs)]

use std::{
    any::TypeId,
    collections::{HashMap, HashSet},
    future::Future,
    io,
    net::{SocketAddr, ToSocketAddrs},
    sync::{Arc, Mutex},
    time::Duration,
};

pub use self::config::Config;

mod config;
mod context;
pub mod fs;
pub mod net;
pub mod plugin;
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
    rand: rand::RandHandle,
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
        #[cfg(feature = "logger")]
        crate::init_logger();

        let rand = rand::RandHandle::new_with_seed(seed);
        let task = task::Executor::new();
        let handle = Handle {
            hosts: Default::default(),
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
        // create host for supervisor
        sim.create("0.0.0.0:0".parse().unwrap());
        sims.insert(TypeId::of::<S>(), sim);
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
    ///
    /// This runs the given future on the current thread until it is complete.
    ///
    /// # Example
    ///
    /// ```
    /// # use madsim_sim as madsim;
    /// use madsim::Runtime;
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
    /// # use madsim_sim as madsim;
    /// use madsim::Runtime;
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
    /// # use madsim_sim as madsim;
    /// use madsim::{Runtime, time::{sleep, Duration}};
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

    /// Enable determinism check during the simulation.
    ///
    /// # Example
    ///
    /// ```should_panic
    /// # use madsim_sim as madsim;
    /// use madsim::{Runtime, time::{sleep, Duration}};
    /// use rand::Rng;
    ///
    /// let f = || async {
    ///     for _ in 0..10 {
    ///         madsim::rand::rng().gen::<u64>();
    ///         // introduce non-determinism
    ///         let rand_num = rand::thread_rng().gen_range(0..10);
    ///         sleep(Duration::from_nanos(rand_num)).await;
    ///     }
    /// };
    ///
    /// let mut rt = Runtime::new();
    /// rt.enable_determinism_check(None);      // enable log
    /// rt.block_on(f());
    /// let log = rt.take_rand_log();           // take log for next turn
    ///
    /// let mut rt = Runtime::new();
    /// rt.enable_determinism_check(log);       // enable check
    /// rt.block_on(f());                       // run the same logic again,
    ///                                         // should panic here.
    /// ```
    pub fn enable_determinism_check(&self, log: Option<rand::Log>) {
        assert_eq!(
            self.task.time_handle().elapsed(),
            Duration::default(),
            "deterministic check must be set at init"
        );
        if let Some(log) = log {
            self.rand.enable_check(log);
        } else {
            self.rand.enable_log();
        }
    }

    /// Take random log so that you can check determinism in the next turn.
    pub fn take_rand_log(self) -> Option<rand::Log> {
        self.rand.take_log()
    }
}

/// Supervisor handle to the runtime.
#[derive(Clone)]
pub struct Handle {
    hosts: Arc<Mutex<HashSet<SocketAddr>>>,
    rand: rand::RandHandle,
    time: time::TimeHandle,
    task: task::TaskHandle,
    sims: Arc<Mutex<HashMap<TypeId, Arc<dyn plugin::Simulator>>>>,
    config: Config,
}

impl Handle {
    /// Returns a [`Handle`] view over the currently running [`Runtime`].
    ///
    /// ## Panic
    ///
    /// This will panic if called outside the context of a Madsim runtime.
    ///
    /// ```should_panic
    /// # use madsim_sim as madsim;
    /// let handle = madsim::Handle::current();
    /// ```
    pub fn current() -> Self {
        context::current(|h| h.clone())
    }

    /// Kill a host.
    ///
    /// - All tasks spawned on this host will be killed immediately.
    /// - All data that has not been flushed to the disk will be lost.
    pub fn kill(&self, addr: SocketAddr) {
        self.hosts.lock().unwrap().remove(&addr);
        self.task.kill(addr);
        for sim in self.sims.lock().unwrap().values() {
            sim.reset(addr);
        }
    }

    /// Restart a host。
    pub fn restart(&self, addr: SocketAddr) {
        self.task.restart(addr);
        for sim in self.sims.lock().unwrap().values() {
            sim.reset(addr);
        }
    }

    /// Pause the execution of a host.
    pub fn pause(&self, addr: SocketAddr) {
        self.task.pause(addr);
    }

    /// Resume the execution of a host.
    pub fn resume(&self, addr: SocketAddr) {
        self.task.resume(addr);
    }

    /// Create a host which will be bound to the specified address.
    pub fn create_host(&self, addr: impl ToSocketAddrs) -> HostBuilder<'_> {
        let addr = addr.to_socket_addrs().unwrap().next().unwrap();
        assert_ne!(addr, SocketAddr::from(([0, 0, 0, 0], 0)), "invalid address");
        if !self.hosts.lock().unwrap().insert(addr) {
            panic!("host already exists: {}", addr);
        }
        HostBuilder::new(self, addr)
    }

    /// Return a handle of the specified host.
    pub fn get_host(&self, addr: SocketAddr) -> Option<LocalHandle> {
        if !self.hosts.lock().unwrap().contains(&addr) {
            return None;
        }
        Some(LocalHandle {
            task: self.task.get_host(addr).unwrap(),
        })
    }
}

/// Builds a host with custom configurations.
pub struct HostBuilder<'a> {
    handle: &'a Handle,
    addr: SocketAddr,
    name: Option<String>,
    init: Option<Arc<dyn Fn(&task::TaskLocalHandle)>>,
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
        self.init = Some(Arc::new(move |handle| {
            handle.spawn_local(future()).detach();
        }));
        self
    }

    /// Build a host.
    pub fn build(self) -> io::Result<LocalHandle> {
        let addr = self.addr;
        let name = self.name.unwrap_or_else(|| addr.to_string());
        for sim in self.handle.sims.lock().unwrap().values() {
            sim.create(addr);
        }
        Ok(LocalHandle {
            task: self.handle.task.create_host(addr, name, self.init),
        })
    }
}

/// Local host handle to the runtime.
#[derive(Clone)]
pub struct LocalHandle {
    task: task::TaskLocalHandle,
}

impl LocalHandle {
    /// Spawn a future onto the runtime.
    pub fn spawn<F>(&self, future: F) -> async_task::Task<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        self.task.spawn(future)
    }

    /// To match the API exposed by std
    pub async fn terminate(&mut self) {
        todo!()
    }
}

#[cfg(feature = "logger")]
fn init_logger() {
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
            if let Some(time) = TimeHandle::try_current() {
                if matches!(start, Some(t0) if time.elapsed() < t0) {
                    return write!(buf, "");
                }
                let task = crate::context::current_task().unwrap();
                writeln!(
                    buf,
                    "{}{:>5}{}{:.6}s{}{}{}{:>10}{} {}",
                    style.value('['),
                    level_style.value(record.level()),
                    style.value("]["),
                    time.elapsed().as_secs_f64(),
                    style.value("]["),
                    task.name,
                    style.value("]["),
                    record.target(),
                    style.value(']'),
                    record.args()
                )
            } else {
                writeln!(
                    buf,
                    "{}{:>5}{}{:>10}{} {}",
                    style.value('['),
                    level_style.value(record.level()),
                    style.value("]["),
                    record.target(),
                    style.value(']'),
                    record.args()
                )
            }
        });
        builder.init();
    });
}
