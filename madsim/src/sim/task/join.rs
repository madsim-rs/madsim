use super::*;

/// An owned permission to join on a task (await its termination).
pub struct JoinHandle<T> {
    abort: AbortHandle,
    /// The task handle.
    ///
    /// This should always be `Some` until drop.
    task: Option<FallibleTask<T>>,
}

impl<T> fmt::Debug for JoinHandle<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("JoinHandle")
            .field("id", &self.id())
            .finish()
    }
}

impl<T> JoinHandle<T> {
    pub(super) fn new(info: Arc<TaskInfo>, task: FallibleTask<T>) -> Self {
        Self {
            abort: AbortHandle { info },
            task: Some(task),
        }
    }

    /// Abort the task associated with the handle.
    pub fn abort(&self) {
        self.abort.abort();
    }

    /// Returns a new [`AbortHandle`] that can be used to remotely abort this task.
    pub fn abort_handle(&self) -> AbortHandle {
        self.abort.clone()
    }

    /// Returns a task ID that uniquely identifies this task relative to other currently spawned tasks.
    pub fn id(&self) -> Id {
        self.abort.id()
    }

    /// Checks if the task associated with this `JoinHandle` has finished.
    pub fn is_finished(&self) -> bool {
        self.task.as_ref().unwrap().is_finished()
    }

    /// Cancel the task when this handle is dropped.
    pub fn cancel_on_drop(mut self) -> FallibleTask<T> {
        self.task.take().unwrap()
    }
}

impl<T> Future for JoinHandle<T> {
    type Output = Result<T, JoinError>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.task.as_mut().unwrap().poll_unpin(cx) {
            Poll::Ready(Some(v)) => Poll::Ready(Ok(v)),
            Poll::Ready(None) => Poll::Ready(Err(JoinError {
                id: self.id(),
                // TODO: indicate if the task panicked
                is_panic: false,
            })),
            Poll::Pending => Poll::Pending,
        }
    }
}

impl<T> Drop for JoinHandle<T> {
    fn drop(&mut self) {
        if let Some(task) = self.task.take() {
            task.detach();
        }
    }
}

/// Task failed to execute to completion.
#[derive(Debug)]
pub struct JoinError {
    id: Id,
    is_panic: bool,
}

impl JoinError {
    /// Returns a task ID that identifies the task which errored relative to other currently spawned tasks.
    pub fn id(&self) -> Id {
        self.id
    }

    /// Returns true if the error was caused by the task being cancelled.
    pub fn is_cancelled(&self) -> bool {
        !self.is_panic
    }

    /// Returns true if the error was caused by the task panicking.
    pub fn is_panic(&self) -> bool {
        self.is_panic
    }
}

impl fmt::Display for JoinError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.is_panic {
            false => write!(f, "task {} was cancelled", self.id),
            true => write!(f, "task {} panicked", self.id),
        }
    }
}

impl std::error::Error for JoinError {}

impl From<JoinError> for io::Error {
    fn from(src: JoinError) -> io::Error {
        io::Error::new(
            io::ErrorKind::Other,
            match src.is_panic {
                false => "task was cancelled",
                true => "task panicked",
            },
        )
    }
}

/// An owned permission to abort a spawned task, without awaiting its completion.
#[derive(Clone)]
pub struct AbortHandle {
    info: Arc<TaskInfo>,
}

impl AbortHandle {
    /// Returns a task ID that uniquely identifies this task relative to other currently spawned tasks.
    pub fn id(&self) -> Id {
        self.info.id
    }

    /// Abort the task associated with the handle.
    pub fn abort(&self) {
        self.info.cancelled.store(true, Ordering::Relaxed);
        self.info.waker.wake_by_ref();
    }
}
