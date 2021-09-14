//! Asynchronous tasks executor.

use super::time::{TimeHandle, TimeRuntime};
use async_task::Runnable;
pub use async_task::Task;
use std::{
    collections::HashMap,
    future::Future,
    net::SocketAddr,
    ops::Deref,
    pin::Pin,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc, Mutex,
    },
    task::{Context, Poll},
    time::Duration,
};

pub(crate) struct Executor {
    queue: mpsc::Receiver<(Runnable, Arc<TaskInfo>)>,
    handle: TaskHandle,
    time: TimeRuntime,
    time_limit: Option<Duration>,
}

#[derive(Debug)]
struct TaskInfo {
    addr: SocketAddr,
    killed: AtomicBool,
}

impl Executor {
    pub fn new() -> Self {
        let (sender, queue) = mpsc::channel();
        Executor {
            queue,
            handle: TaskHandle {
                info: Arc::new(Mutex::new(HashMap::new())),
                sender,
            },
            time: TimeRuntime::new(),
            time_limit: None,
        }
    }

    pub fn handle(&self) -> &TaskHandle {
        &self.handle
    }

    pub fn time_handle(&self) -> &TimeHandle {
        self.time.handle()
    }

    pub fn set_time_limit(&mut self, limit: Duration) {
        self.time_limit = Some(limit);
    }

    pub fn block_on<F: Future>(&self, future: F) -> F::Output {
        let sender = self.handle.sender.clone();
        let info = Arc::new(TaskInfo {
            addr: "0.0.0.0:0".parse().unwrap(),
            killed: AtomicBool::new(false),
        });
        let (runnable, mut task) = unsafe {
            // Safety: The schedule is not Sync,
            // the task's Waker must be used and dropped on the original thread.
            async_task::spawn_unchecked(future, move |runnable| {
                sender.send((runnable, info.clone())).unwrap();
            })
        };
        runnable.schedule();

        // empty context to poll the result
        let waker = futures::task::noop_waker();
        let mut cx = Context::from_waker(&waker);

        loop {
            self.run_all_ready();
            if let Poll::Ready(val) = Pin::new(&mut task).poll(&mut cx) {
                return val;
            }
            let going = self.time.advance();
            assert!(going, "no events, the task will block forever");
            if let Some(limit) = self.time_limit {
                assert!(
                    self.time.handle().elapsed() < limit,
                    "time limit exceeded: {:?}",
                    limit
                )
            }
        }
    }

    fn run_all_ready(&self) {
        while let Ok((runnable, info)) = self.queue.try_recv() {
            if info.killed.load(Ordering::SeqCst) {
                continue;
            }
            let _guard = crate::context::enter_task(info.addr);
            runnable.run();
        }
    }
}

impl Deref for Executor {
    type Target = TaskHandle;

    fn deref(&self) -> &Self::Target {
        &self.handle
    }
}

#[derive(Clone)]
pub(crate) struct TaskHandle {
    sender: mpsc::Sender<(Runnable, Arc<TaskInfo>)>,
    info: Arc<Mutex<HashMap<SocketAddr, Arc<TaskInfo>>>>,
}

impl TaskHandle {
    /// Kill all tasks of the address.
    pub fn kill(&self, addr: SocketAddr) {
        let mut info = self.info.lock().unwrap();
        if let Some(info) = info.remove(&addr) {
            info.killed.store(true, Ordering::SeqCst);
        }
    }

    pub fn local_handle(&self, addr: SocketAddr) -> TaskLocalHandle {
        let mut info = self.info.lock().unwrap();
        let info = info
            .entry(addr)
            .or_insert_with(|| {
                Arc::new(TaskInfo {
                    addr,
                    killed: AtomicBool::new(false),
                })
            })
            .clone();
        TaskLocalHandle {
            sender: self.sender.clone(),
            info,
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct TaskLocalHandle {
    sender: mpsc::Sender<(Runnable, Arc<TaskInfo>)>,
    info: Arc<TaskInfo>,
}

impl TaskLocalHandle {
    pub fn spawn<F>(&self, future: F) -> Task<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        self.spawn_local(future)
    }

    pub fn spawn_local<F>(&self, future: F) -> Task<F::Output>
    where
        F: Future + 'static,
        F::Output: 'static,
    {
        let sender = self.sender.clone();
        let info = self.info.clone();
        let (runnable, task) = unsafe {
            // Safety: The schedule is not Sync,
            // the task's Waker must be used and dropped on the original thread.
            async_task::spawn_unchecked(future, move |runnable| {
                let _ = sender.send((runnable, info.clone()));
            })
        };
        runnable.schedule();
        task
    }
}

/// Spawns a new asynchronous task, returning a [`Task`] for it.
pub fn spawn<F>(future: F) -> Task<F::Output>
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    let handle = crate::context::task_local_handle();
    handle.spawn(future)
}

/// Spawns a `!Send` future on the local task set.
pub fn spawn_local<F>(future: F) -> Task<F::Output>
where
    F: Future + 'static,
    F::Output: 'static,
{
    let handle = crate::context::task_local_handle();
    handle.spawn_local(future)
}

/// Runs the provided closure on a thread where blocking is acceptable.
pub fn spawn_blocking<F, R>(f: F) -> Task<R>
where
    F: FnOnce() -> R + Send + 'static,
    R: Send + 'static,
{
    let handle = crate::context::task_local_handle();
    handle.spawn(async move { f() })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{time, Runtime};
    use std::{sync::atomic::AtomicUsize, time::Duration};

    #[test]
    fn kill() {
        let runtime = Runtime::new();
        let host1 = runtime.create_host("0.0.0.1:1").unwrap();
        let host2 = runtime.create_host("0.0.0.2:1").unwrap();
        let addr1 = host1.local_addr();

        let flag1 = Arc::new(AtomicUsize::new(0));
        let flag2 = Arc::new(AtomicUsize::new(0));

        let flag1_ = flag1.clone();
        host1
            .spawn(async move {
                loop {
                    time::sleep(Duration::from_secs(2)).await;
                    flag1_.fetch_add(2, Ordering::SeqCst);
                }
            })
            .detach();

        let flag2_ = flag2.clone();
        host2
            .spawn(async move {
                loop {
                    time::sleep(Duration::from_secs(2)).await;
                    flag2_.fetch_add(2, Ordering::SeqCst);
                }
            })
            .detach();

        let handle = runtime.handle();
        runtime.block_on(async move {
            let t0 = time::Instant::now();

            time::sleep_until(t0 + Duration::from_secs(3)).await;
            assert_eq!(flag1.load(Ordering::SeqCst), 2);
            assert_eq!(flag2.load(Ordering::SeqCst), 2);
            handle.task.kill(addr1);

            time::sleep_until(t0 + Duration::from_secs(5)).await;
            assert_eq!(flag1.load(Ordering::SeqCst), 2);
            assert_eq!(flag2.load(Ordering::SeqCst), 4);
        });
    }
}
