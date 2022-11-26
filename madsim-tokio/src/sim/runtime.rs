use madsim::task::JoinHandle;
use std::{future::Future, io};

/// Builds Tokio Runtime with custom configuration values.
pub struct Builder {}

impl Builder {
    pub fn new_current_thread() -> Builder {
        unimplemented!("blocking run is not supported in simulation");
    }

    /// Returns a new builder with the multi thread scheduler selected.
    #[cfg(feature = "rt-multi-thread")]
    #[cfg_attr(docsrs, doc(cfg(feature = "rt-multi-thread")))]
    pub fn new_multi_thread() -> Builder {
        Builder {}
    }

    /// Sets the number of worker threads the `Runtime` will use.
    #[track_caller]
    pub fn worker_threads(&mut self, val: usize) -> &mut Self {
        assert!(val > 0, "Worker threads cannot be set to 0");
        // self.worker_threads = Some(val);
        self
    }

    /// Sets name of threads spawned by the `Runtime`'s thread pool.
    pub fn thread_name(&mut self, _val: impl Into<String>) -> &mut Self {
        // let val = val.into();
        // self.thread_name = std::sync::Arc::new(move || val.clone());
        self
    }

    /// Enables both I/O and time drivers.
    pub fn enable_all(&mut self) -> &mut Self {
        self
    }

    /// Creates the configured `Runtime`.
    pub fn build(&mut self) -> io::Result<Runtime> {
        Ok(Runtime {})
    }
}

/// A fake Tokio runtime.
pub struct Runtime {}

impl Runtime {
    #[cfg(feature = "rt-multi-thread")]
    #[cfg_attr(docsrs, doc(cfg(feature = "rt-multi-thread")))]
    pub fn new() -> io::Result<Runtime> {
        Builder::new_multi_thread().enable_all().build()
    }

    /// Spawns a future onto the Tokio runtime.
    #[track_caller]
    pub fn spawn<F>(&self, future: F) -> JoinHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        madsim::task::spawn(future)
    }
}

impl Drop for Runtime {
    fn drop(&mut self) {
        todo!("drop runtime, abort all tasks");
    }
}
