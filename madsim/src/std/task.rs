//! Asynchronous tasks executor.

use std::future::Future;

/// Spawns a new asynchronous task, returning a [`Task`] for it.
pub fn spawn<F>(future: F) -> JoinHandle<F::Output>
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    JoinHandle(tokio::spawn(future))
}

/// Spawns a `!Send` future on the local task set.
pub fn spawn_local<F>(future: F) -> JoinHandle<F::Output>
where
    F: Future + 'static,
    F::Output: 'static,
{
    JoinHandle(tokio::task::spawn_local(future))
}

/// Runs the provided closure on a thread where blocking is acceptable.
pub fn spawn_blocking<F, R>(f: F) -> JoinHandle<R>
where
    F: FnOnce() -> R + Send + 'static,
    R: Send + 'static,
{
    JoinHandle(tokio::task::spawn_blocking(f))
}

/// A spawned task.
#[must_use = "you must either `.await` or explicitly `.detach()` the task"]
pub struct JoinHandle<T>(tokio::task::JoinHandle<T>);

impl<T> JoinHandle<T> {
    /// Detaches the task to let it keep running in the background.
    pub fn detach(self) {
        std::mem::forget(self);
    }

    /// Abort the task associated with the handle.
    pub fn abort(&mut self) {
        self.0.abort();
    }
}

impl<T> Drop for JoinHandle<T> {
    fn drop(&mut self) {
        self.0.abort();
    }
}

impl<T> Future for JoinHandle<T> {
    type Output = T;

    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        std::pin::Pin::new(&mut self.0).poll(cx).map(|o| o.unwrap())
    }
}
