//! A deterministic simulator for distributed systems.
//!
//! ## Features
//!
//! - `rpc`: Enables RPC through network.
//! - `logger`: Enables built-in logger.
//! - `macros`: Enables `#[madsim::main]` and `#[madsim::test]` macros.

#![deny(missing_docs)]

use std::{future::Future, net::SocketAddr, time::Duration};

mod context;
pub mod fs;
pub mod net;
pub mod rand;
pub mod task;
pub mod time;

#[cfg(feature = "macros")]
pub use madsim_macros::{main, test};

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
    net: net::NetRuntime,
    fs: fs::FsRuntime,
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
    pub fn new_with_seed(seed: u64) -> Self {
        #[cfg(feature = "logger")]
        crate::init_logger();

        let rand = rand::RandHandle::new_with_seed(seed);
        let task = task::Executor::new();
        let net = net::NetRuntime::new(rand.clone(), task.time_handle().clone());
        let fs = fs::FsRuntime::new(rand.clone(), task.time_handle().clone());
        Runtime {
            rand,
            task,
            net,
            fs,
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
            rand: self.rand.clone(),
            time: self.task.time_handle().clone(),
            task: self.task.handle().clone(),
            net: self.net.handle().clone(),
            fs: self.fs.handle().clone(),
        }
    }

    /// Return a handle of the specified host.
    ///
    /// The returned handle can be used to spawn tasks that run on this host.
    pub fn local_handle(&self, addr: SocketAddr) -> LocalHandle {
        assert_ne!(addr, SocketAddr::from(([0, 0, 0, 0], 0)), "invalid address");
        LocalHandle {
            task: self.task.handle().local_handle(addr),
            net: self.net.handle().local_handle(addr),
            fs: self.fs.handle().local_handle(addr),
        }
    }

    /// Set a time limit of the execution.
    ///
    /// The runtime will panic when time limit exceeded.
    /// # Example
    ///
    /// ```should_panic
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

    /// Enable deterministic check during the simulation.
    pub fn enable_deterministic_check(&self, log: Option<Vec<u8>>) {
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

    /// Take random log so that you can check deterministic in the next turn.
    pub fn take_rand_log(self) -> Option<Vec<u8>> {
        self.rand.take_log()
    }

    /// Run a future to completion on the runtime. This is the runtime’s entry point.
    ///
    /// This runs the given future on the current thread until it is complete.
    ///
    /// # Example
    ///
    /// ```
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
    /// use madsim::Runtime;
    /// use futures::future::pending;
    ///
    /// Runtime::new().block_on(pending::<()>());
    /// ```
    pub fn block_on<F: Future>(&self, future: F) -> F::Output {
        let _guard = crate::context::enter(self.handle());
        self.task.block_on(future)
    }
}

/// Supervisor handle to the runtime.
#[derive(Clone)]
#[allow(missing_docs)]
pub struct Handle {
    rand: rand::RandHandle,
    time: time::TimeHandle,
    task: task::TaskHandle,
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
        context::current()
    }

    /// Kill a host.
    ///
    /// - All tasks spawned on this host will be killed immediately.
    /// - All data that has not been flushed to the disk will be lost.
    ///
    /// # Example
    /// ```
    /// use madsim::{Runtime, time::{sleep, Duration}};
    /// use std::sync::{Arc, atomic::{AtomicUsize, Ordering}};
    ///
    /// let rt = Runtime::new();
    /// let addr = "0.0.0.1:1".parse().unwrap();
    ///
    /// // host increases the counter every 2s
    /// let flag = Arc::new(AtomicUsize::new(0));
    /// let flag_ = flag.clone();
    /// rt.local_handle(addr).spawn(async move {
    ///     loop {
    ///         sleep(Duration::from_secs(2)).await;
    ///         flag_.fetch_add(2, Ordering::SeqCst);
    ///     }
    /// }).detach();
    ///  
    /// let handle = rt.handle();
    /// rt.block_on(async move {
    ///     sleep(Duration::from_secs(3)).await;
    ///     assert_eq!(flag.load(Ordering::SeqCst), 2);
    ///
    ///     handle.kill(addr);
    ///
    ///     sleep(Duration::from_secs(2)).await;
    ///     assert_eq!(flag.load(Ordering::SeqCst), 2);
    /// });
    /// ```
    pub fn kill(&self, addr: SocketAddr) {
        self.task.kill(addr);
        // self.net.kill(addr);
        // self.fs.power_fail(addr);
    }

    /// Return a handle of the specified host.
    pub fn local_handle(&self, addr: SocketAddr) -> LocalHandle {
        LocalHandle {
            task: self.task.local_handle(addr),
            net: self.net.local_handle(addr),
            fs: self.fs.local_handle(addr),
        }
    }
}

/// Local host handle to the runtime.
#[derive(Clone)]
pub struct LocalHandle {
    task: task::TaskLocalHandle,
    net: net::NetLocalHandle,
    fs: fs::FsLocalHandle,
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
            if let Some(time) = crate::context::try_time_handle() {
                if matches!(start, Some(t0) if time.elapsed() < t0) {
                    return write!(buf, "");
                }
                let addr = crate::context::current_addr().unwrap();
                writeln!(
                    buf,
                    "{}{:>5}{}{:.6}s{}{}{}{:>10}{} {}",
                    style.value('['),
                    level_style.value(record.level()),
                    style.value("]["),
                    time.elapsed().as_secs_f64(),
                    style.value("]["),
                    addr,
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
