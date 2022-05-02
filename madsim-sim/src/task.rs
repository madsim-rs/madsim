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

pub(crate) struct TaskInfo {
    pub addr: SocketAddr,
    pub name: String,
    /// A flag indicating that the task should be paused.
    paused: AtomicBool,
    /// A flag indicating that the task should no longer be executed.
    killed: AtomicBool,
    /// A function to spawn the initial task.
    init: Option<Arc<dyn Fn(&TaskLocalHandle)>>,
}

impl Executor {
    pub fn new() -> Self {
        let (sender, queue) = mpsc::channel();
        Executor {
            queue,
            handle: TaskHandle {
                hosts: Arc::new(Mutex::new(HashMap::new())),
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
        // push the future into ready queue.
        let sender = self.handle.sender.clone();
        let info = Arc::new(TaskInfo {
            addr: "0.0.0.0:0".parse().unwrap(),
            name: "main".into(),
            paused: AtomicBool::new(false),
            killed: AtomicBool::new(false),
            init: None,
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

    /// Drain all tasks from ready queue and run them.
    fn run_all_ready(&self) {
        while let Ok((runnable, info)) = self.queue.try_recv() {
            if info.killed.load(Ordering::SeqCst) {
                // killed task: ignore
                continue;
            } else if info.paused.load(Ordering::SeqCst) {
                // paused task: push to waiting list
                let mut hosts = self.hosts.lock().unwrap();
                hosts.get_mut(&info.addr).unwrap().paused.push(runnable);
                continue;
            }
            // run task
            let _guard = crate::context::enter_task(info);
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
    hosts: Arc<Mutex<HashMap<SocketAddr, Host>>>,
}

struct Host {
    info: Arc<TaskInfo>,
    paused: Vec<Runnable>,
}

impl TaskHandle {
    /// Kill all tasks of the address.
    pub fn kill(&self, addr: SocketAddr) {
        if let Some(host) = self.hosts.lock().unwrap().remove(&addr) {
            host.info.killed.store(true, Ordering::SeqCst);
        }
    }

    /// Kill all tasks of the address and restart the initial task.
    pub fn restart(&self, addr: SocketAddr) {
        let info = self
            .hosts
            .lock()
            .unwrap()
            .remove(&addr)
            .expect("host not found")
            .info;
        info.killed.store(true, Ordering::SeqCst);
        self.create_host(info.addr, info.name.clone(), info.init.clone());
    }

    /// Pause all tasks of the address.
    pub fn pause(&self, addr: SocketAddr) {
        let hosts = self.hosts.lock().unwrap();
        let host = hosts.get(&addr).expect("host not found");
        host.info.paused.store(true, Ordering::SeqCst);
    }

    /// Resume the execution of the address.
    pub fn resume(&self, addr: SocketAddr) {
        let mut hosts = self.hosts.lock().unwrap();
        let host = hosts.get_mut(&addr).expect("host not found");
        host.info.paused.store(false, Ordering::SeqCst);

        // take paused tasks from waiting list and push them to ready queue
        for runnable in host.paused.drain(..) {
            self.sender.send((runnable, host.info.clone())).unwrap();
        }
    }

    /// Create a new host.
    pub fn create_host(
        &self,
        addr: SocketAddr,
        name: String,
        init: Option<Arc<dyn Fn(&TaskLocalHandle)>>,
    ) -> TaskLocalHandle {
        let info = Arc::new(TaskInfo {
            addr,
            name,
            paused: AtomicBool::new(false),
            killed: AtomicBool::new(false),
            init: init.clone(),
        });
        let host = Host {
            info: info.clone(),
            paused: vec![],
        };
        self.hosts.lock().unwrap().insert(addr, host);

        let handle = TaskLocalHandle {
            sender: self.sender.clone(),
            info,
        };
        if let Some(init) = &init {
            init(&handle);
        }
        handle
    }

    /// Get the host handle.
    pub fn get_host(&self, addr: SocketAddr) -> Option<TaskLocalHandle> {
        let hosts = self.hosts.lock().unwrap();
        let info = hosts.get(&addr)?.info.clone();
        Some(TaskLocalHandle {
            sender: self.sender.clone(),
            info,
        })
    }
}

#[derive(Clone)]
pub(crate) struct TaskLocalHandle {
    sender: mpsc::Sender<(Runnable, Arc<TaskInfo>)>,
    info: Arc<TaskInfo>,
}

impl TaskLocalHandle {
    fn current() -> Self {
        let addr = crate::context::current_addr();
        crate::context::current(|h| h.task.get_host(addr).unwrap())
    }

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
    let handle = TaskLocalHandle::current();
    handle.spawn(future)
}

/// Spawns a `!Send` future on the local task set.
pub fn spawn_local<F>(future: F) -> Task<F::Output>
where
    F: Future + 'static,
    F::Output: 'static,
{
    let handle = TaskLocalHandle::current();
    handle.spawn_local(future)
}

/// Runs the provided closure on a thread where blocking is acceptable.
pub fn spawn_blocking<F, R>(f: F) -> Task<R>
where
    F: FnOnce() -> R + Send + 'static,
    R: Send + 'static,
{
    let handle = TaskLocalHandle::current();
    handle.spawn(async move { f() })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{time, Handle, Runtime};
    use std::{sync::atomic::AtomicUsize, time::Duration};

    #[test]
    fn kill() {
        let runtime = Runtime::new();
        let addr1 = "0.0.0.1:1".parse::<SocketAddr>().unwrap();
        let addr2 = "0.0.0.2:1".parse::<SocketAddr>().unwrap();
        let host1 = runtime.create_host(addr1).build().unwrap();
        let host2 = runtime.create_host(addr2).build().unwrap();

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

        runtime.block_on(async move {
            let t0 = time::Instant::now();

            time::sleep_until(t0 + Duration::from_secs(3)).await;
            assert_eq!(flag1.load(Ordering::SeqCst), 2);
            assert_eq!(flag2.load(Ordering::SeqCst), 2);
            Handle::current().kill(addr1);

            time::sleep_until(t0 + Duration::from_secs(5)).await;
            assert_eq!(flag1.load(Ordering::SeqCst), 2);
            assert_eq!(flag2.load(Ordering::SeqCst), 4);
        });
    }

    #[test]
    fn restart() {
        let runtime = Runtime::new();

        let flag = Arc::new(AtomicUsize::new(0));

        let addr1 = "0.0.0.1:1".parse::<SocketAddr>().unwrap();
        let flag_ = flag.clone();
        runtime
            .create_host(addr1)
            .init(move || {
                let flag = flag_.clone();
                async move {
                    // set flag to 0, then +2 every 2s
                    flag.store(0, Ordering::SeqCst);
                    loop {
                        time::sleep(Duration::from_secs(2)).await;
                        flag.fetch_add(2, Ordering::SeqCst);
                    }
                }
            })
            .build()
            .unwrap();

        runtime.block_on(async move {
            let t0 = time::Instant::now();

            time::sleep_until(t0 + Duration::from_secs(3)).await;
            assert_eq!(flag.load(Ordering::SeqCst), 2);
            Handle::current().restart(addr1);

            time::sleep_until(t0 + Duration::from_secs(6)).await;
            assert_eq!(flag.load(Ordering::SeqCst), 2);

            time::sleep_until(t0 + Duration::from_secs(8)).await;
            assert_eq!(flag.load(Ordering::SeqCst), 4);
        });
    }

    #[test]
    fn pause_resume() {
        let runtime = Runtime::new();
        let addr1 = "0.0.0.1:1".parse::<SocketAddr>().unwrap();
        let host1 = runtime.create_host(addr1).build().unwrap();

        let flag1 = Arc::new(AtomicUsize::new(0));
        let flag1_ = flag1.clone();
        host1
            .spawn(async move {
                loop {
                    time::sleep(Duration::from_secs(2)).await;
                    flag1_.fetch_add(2, Ordering::SeqCst);
                }
            })
            .detach();

        runtime.block_on(async move {
            let t0 = time::Instant::now();

            time::sleep_until(t0 + Duration::from_secs(3)).await;
            assert_eq!(flag1.load(Ordering::SeqCst), 2);
            Handle::current().pause(addr1);
            Handle::current().pause(addr1);

            time::sleep_until(t0 + Duration::from_secs(5)).await;
            assert_eq!(flag1.load(Ordering::SeqCst), 2);

            Handle::current().resume(addr1);
            Handle::current().resume(addr1);
            time::sleep_until(t0 + Duration::from_secs_f32(5.5)).await;
            assert_eq!(flag1.load(Ordering::SeqCst), 4);
        });
    }
}
