//! Asynchronous tasks executor.

use std::future::Future;

/// Spawns a new asynchronous task, returning a [`Task`] for it.
pub fn spawn<F>(future: F) -> Task<F::Output>
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    Task(tokio::spawn(future))
}

/// Spawns a `!Send` future on the local task set.
pub fn spawn_local<F>(future: F) -> Task<F::Output>
where
    F: Future + 'static,
    F::Output: 'static,
{
    Task(tokio::task::spawn_local(future))
}

/// Runs the provided closure on a thread where blocking is acceptable.
pub fn spawn_blocking<F, R>(f: F) -> Task<R>
where
    F: FnOnce() -> R + Send + 'static,
    R: Send + 'static,
{
    Task(tokio::task::spawn_blocking(f))
}

/// A spawned task.
pub struct Task<T>(pub(crate) tokio::task::JoinHandle<T>);

/// A spawned task.
pub type JoinHandle<T> = Task<T>;

impl<T> Task<T> {
    /// Detaches the task to let it keep running in the background.
    pub fn detach(self) {
        std::mem::forget(self);
    }

    /// Cancels the task and waits for it to stop running.
    pub async fn cancel(self) -> Option<T> {
        // TODO: get output if ready
        self.0.abort();
        None
    }
}

impl<T> Drop for Task<T> {
    fn drop(&mut self) {
        self.0.abort();
    }
}

impl<T> Future for Task<T> {
    type Output = T;

    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        std::pin::Pin::new(&mut self.0).poll(cx).map(|o| o.unwrap())
    }
}
