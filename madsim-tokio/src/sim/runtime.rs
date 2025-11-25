use madsim::runtime::Handle as MadsimHandle;
use madsim::task::{AbortHandle, JoinHandle};
use spin::Mutex;
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
        Ok(Runtime {
            abort_handles: Default::default(),
            handle: Handle::current(),
        })
    }
}

/// A fake Tokio runtime.
pub struct Runtime {
    abort_handles: Mutex<Vec<AbortHandle>>,
    handle: Handle,
}

#[allow(dead_code)]
pub struct EnterGuard<'a>(&'a Handle);

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
        let handle = madsim::task::spawn(future);
        self.abort_handles.lock().push(handle.abort_handle());
        handle
    }

    /// Runs the provided function on an executor dedicated to blocking operations.
    #[track_caller]
    pub fn spawn_blocking<F, R>(&self, func: F) -> JoinHandle<R>
    where
        F: FnOnce() -> R + Send + 'static,
        R: Send + 'static,
    {
        #[allow(deprecated)]
        let handle = madsim::task::spawn_blocking(func);
        self.abort_handles.lock().push(handle.abort_handle());
        handle
    }

    pub fn block_on<F: Future>(&self, _future: F) -> F::Output {
        unimplemented!("blocking the current thread is not allowed in madsim");
    }

    pub fn enter(&self) -> EnterGuard<'_> {
        // Madsim runtime is entered by default. No-op here.
        EnterGuard(&self.handle)
    }

    /// Returns a handle to the runtime’s spawner.
    pub fn handle(&self) -> &Handle {
        &self.handle
    }
}

impl Drop for Runtime {
    fn drop(&mut self) {
        for handle in self.abort_handles.lock().drain(..) {
            handle.abort();
        }
    }
}

pub struct TryCurrentError;

/// Handle to the tokio runtime.
///
/// FIXME: tasks spawned with this handle are not correctly associated with the tokio runtime.
#[derive(Debug, Clone)]
pub struct Handle;

impl Handle {
    /// Returns a handle to the current runtime.
    pub fn current() -> Self {
        Handle
    }

    /// Returns a handle to the current runtime.
    pub fn try_current() -> Result<Self, TryCurrentError> {
        match MadsimHandle::try_current() {
            Ok(_) => Ok(Handle),
            Err(_e) => Err(TryCurrentError),
        }
    }

    /// Enters the runtime context.
    ///
    /// FIXME: This is currently a no-op.
    pub fn enter(&self) -> EnterGuard<'_> {
        EnterGuard(self)
    }

    /// Spawns a future onto the Tokio runtime.
    ///
    /// FIXME: tasks spawned with this handle are not correctly associated with the tokio runtime.
    #[track_caller]
    pub fn spawn<F>(&self, future: F) -> JoinHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        madsim::task::spawn(future)
    }

    /// Runs the provided function on an executor dedicated to blocking operations.
    ///
    /// FIXME: tasks spawned with this handle are not correctly associated with the tokio runtime.
    #[track_caller]
    pub fn spawn_blocking<F, R>(&self, func: F) -> JoinHandle<R>
    where
        F: FnOnce() -> R + Send + 'static,
        R: Send + 'static,
    {
        #[allow(deprecated)]
        madsim::task::spawn_blocking(func)
    }
}

/// Dummy tokio runtime metrics.
///
/// This is only provided to provide API compatibility. Metric values are always zero.
#[derive(Clone)]
pub struct RuntimeMetrics;

impl RuntimeMetrics {
    /// Returns the number of worker threads used by the runtime.
    pub fn num_workers(&self) -> usize {
        0
    }

    /// Returns the current number of alive tasks in the runtime.
    pub fn num_alive_tasks(&self) -> usize {
        0
    }

    /// Returns the number of tasks currently scheduled in the runtime’s global queue.
    pub fn global_queue_depth(&self) -> usize {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_drop() {
        let runtime = madsim::runtime::Runtime::new();

        runtime.block_on(async move {
            let rt = Runtime::new().unwrap();
            let handle = rt.handle().clone();
            let join_handle = rt.spawn(std::future::pending::<()>());
            let _join_handle2 = handle.spawn(std::future::pending::<()>());
            drop(rt);

            let err = join_handle.await.unwrap_err();
            assert!(err.is_cancelled());
            // FIXME: task spawned by the handle should also be cancelled.
            // let err = join_handle2.await.unwrap_err();
            // assert!(err.is_cancelled());
        });
    }
}
