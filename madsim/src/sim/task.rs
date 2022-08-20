//! Asynchronous tasks executor.

use super::{
    rand::GlobalRng,
    time::{TimeHandle, TimeRuntime},
    utils::mpsc,
};
use async_task::{FallibleTask, Runnable};
use rand::Rng;
use spin::Mutex;
use std::{
    collections::HashMap,
    fmt,
    future::Future,
    io,
    ops::Deref,
    pin::Pin,
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc,
    },
    task::{Context, Poll},
    time::Duration,
};
use tracing::*;

pub use tokio::task::yield_now;

pub(crate) struct Executor {
    queue: mpsc::Receiver<(Runnable, Arc<TaskInfo>)>,
    handle: TaskHandle,
    rand: GlobalRng,
    time: TimeRuntime,
    time_limit: Option<Duration>,
}

/// A unique identifier for a node.
#[cfg_attr(docsrs, doc(cfg(madsim)))]
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy)]
pub struct NodeId(u64);

impl fmt::Display for NodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl NodeId {
    pub(crate) const fn zero() -> Self {
        NodeId(0)
    }
}

pub(crate) struct TaskInfo {
    pub id: Id,
    // name: Option<String>,
    pub node: Arc<NodeInfo>,
    /// The span of this task.
    span: Span,
}

pub(crate) struct NodeInfo {
    pub id: NodeId,
    pub name: String,
    /// The number of CPU cores.
    pub cores: usize,
    /// A flag indicating that the task should be paused.
    paused: AtomicBool,
    /// A flag indicating that the task should no longer be executed.
    killed: AtomicBool,
    /// The span of this node.
    span: Span,
}

impl NodeInfo {
    fn new_task(self: &Arc<Self>, name: Option<&str>) -> Arc<TaskInfo> {
        let id = Id::new();
        let name = name.map(|s| s.to_string());
        Arc::new(TaskInfo {
            span: error_span!(parent: &self.span, "task", %id, name),
            id,
            // name,
            node: self.clone(),
        })
    }
}

impl Executor {
    pub fn new(rand: GlobalRng) -> Self {
        let (sender, queue) = mpsc::channel();
        Executor {
            queue,
            handle: TaskHandle {
                nodes: Arc::new(Mutex::new(HashMap::new())),
                sender,
                next_node_id: Arc::new(AtomicU64::new(1)),
                main_info: Arc::new(NodeInfo {
                    id: NodeId::zero(),
                    name: "main".into(),
                    cores: 1,
                    paused: AtomicBool::new(false),
                    killed: AtomicBool::new(false),
                    span: error_span!("node", id = %NodeId::zero(), name = "main"),
                }),
            },
            time: TimeRuntime::new(&rand),
            rand,
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
        let info = self.handle.main_info.new_task(None);
        let (runnable, mut task) = unsafe {
            // Safety: The schedule is not Sync,
            // the task's Waker must be used and dropped on the original thread.
            async_task::spawn_unchecked(future, move |runnable| {
                let _ = sender.send((runnable, info.clone()));
            })
        };

        let waker = runnable.waker();
        runnable.schedule();

        let mut cx = Context::from_waker(&waker);

        loop {
            self.run_all_ready();
            if let Poll::Ready(val) = Pin::new(&mut task).poll(&mut cx) {
                return val;
            }
            let going = self.time.advance_to_next_event();
            assert!(going, "no events, all tasks will block forever");
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
        while let Ok((runnable, info)) = self.queue.try_recv_random(&self.rand) {
            if info.node.killed.load(Ordering::SeqCst) {
                // killed task: ignore
                continue;
            } else if info.node.paused.load(Ordering::SeqCst) {
                // paused task: push to waiting list
                let mut nodes = self.nodes.lock();
                nodes
                    .get_mut(&info.node.id)
                    .unwrap()
                    .paused
                    .push((runnable, info));
                continue;
            }
            // run the task
            let _enter = info.span.clone().entered();
            let _guard = crate::context::enter_task(info);
            runnable.run();

            // advance time: 50-100ns
            let dur = Duration::from_nanos(self.rand.with(|rng| rng.gen_range(50..100)));
            self.time.advance(dur);
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
#[doc(hidden)]
pub struct TaskHandle {
    sender: mpsc::Sender<(Runnable, Arc<TaskInfo>)>,
    nodes: Arc<Mutex<HashMap<NodeId, Node>>>,
    next_node_id: Arc<AtomicU64>,
    /// Info of the main node.
    main_info: Arc<NodeInfo>,
}

struct Node {
    info: Arc<NodeInfo>,
    paused: Vec<(Runnable, Arc<TaskInfo>)>,
    /// A function to spawn the initial task.
    init: Option<InitFn>,
}

pub(crate) type InitFn = Arc<dyn Fn(&Spawner)>;

impl TaskHandle {
    /// Kill all tasks of the node.
    pub fn kill(&self, id: impl ToNodeId) {
        debug!(node = %id, "kill");
        let id = id.to_node_id(self);
        self.kill_id(id);
    }

    fn kill_id(&self, id: NodeId) {
        let mut nodes = self.nodes.lock();
        let node = nodes.get_mut(&id).expect("node not found");
        node.paused.clear();
        let new_info = Arc::new(NodeInfo {
            id,
            name: node.info.name.clone(),
            cores: 1,
            paused: AtomicBool::new(false),
            killed: AtomicBool::new(false),
            span: error_span!(parent: None, "node", %id, name = &node.info.name),
        });
        let old_info = std::mem::replace(&mut node.info, new_info);
        old_info.killed.store(true, Ordering::SeqCst);
    }

    /// Kill all tasks of the node and restart the initial task.
    pub fn restart(&self, id: impl ToNodeId) {
        debug!(node = %id, "restart");
        let id = id.to_node_id(self);
        self.kill_id(id);
        let nodes = self.nodes.lock();
        let node = nodes.get(&id).expect("node not found");
        if let Some(init) = &node.init {
            init(&Spawner {
                sender: self.sender.clone(),
                info: node.info.clone(),
            });
        }
    }

    /// Pause all tasks of the node.
    pub fn pause(&self, id: impl ToNodeId) {
        debug!(node = %id, "pause");
        let id = id.to_node_id(self);
        let nodes = self.nodes.lock();
        let node = nodes.get(&id).expect("node not found");
        node.info.paused.store(true, Ordering::SeqCst);
    }

    /// Resume the execution of the address.
    pub fn resume(&self, id: impl ToNodeId) {
        debug!(node = %id, "resume");
        let id = id.to_node_id(self);
        let mut nodes = self.nodes.lock();
        let node = nodes.get_mut(&id).expect("node not found");
        node.info.paused.store(false, Ordering::SeqCst);

        // take paused tasks from waiting list and push them to ready queue
        for (runnable, info) in node.paused.drain(..) {
            self.sender.send((runnable, info)).unwrap();
        }
    }

    /// Create a new node.
    pub fn create_node(
        &self,
        name: Option<String>,
        init: Option<InitFn>,
        cores: Option<usize>,
    ) -> Spawner {
        let id = NodeId(self.next_node_id.fetch_add(1, Ordering::SeqCst));
        let name = name.unwrap_or_else(|| format!("node-{}", id.0));
        debug!(node = %id, name, "create");
        let info = Arc::new(NodeInfo {
            span: error_span!(parent: None, "node", %id, name),
            id,
            name,
            cores: cores.unwrap_or(1),
            paused: AtomicBool::new(false),
            killed: AtomicBool::new(false),
        });
        let handle = Spawner {
            sender: self.sender.clone(),
            info: info.clone(),
        };
        if let Some(init) = &init {
            init(&handle);
        }
        let node = Node {
            info,
            paused: vec![],
            init,
        };
        self.nodes.lock().insert(id, node);
        handle
    }

    /// Get the node handle.
    pub fn get_node(&self, id: impl ToNodeId) -> Option<Spawner> {
        let id = id.to_node_id(self);
        let info = match id {
            NodeId(0) => self.main_info.clone(),
            _ => self.nodes.lock().get(&id)?.info.clone(),
        };
        Some(Spawner {
            sender: self.sender.clone(),
            info,
        })
    }
}

/// A trait for objects which can be resolved to an identifier of node.
pub trait ToNodeId: std::fmt::Display {
    #[doc(hidden)]
    fn to_node_id(&self, task: &TaskHandle) -> NodeId;
}

impl ToNodeId for NodeId {
    fn to_node_id(&self, _task: &TaskHandle) -> NodeId {
        *self
    }
}

impl ToNodeId for &str {
    fn to_node_id(&self, task: &TaskHandle) -> NodeId {
        match (task.nodes.lock().iter()).find(|(_, node)| &node.info.name == self) {
            Some((id, _)) => *id,
            None => panic!("node not found: {self}"),
        }
    }
}

impl ToNodeId for String {
    fn to_node_id(&self, task: &TaskHandle) -> NodeId {
        self.as_str().to_node_id(task)
    }
}

impl<T: ToNodeId> ToNodeId for &T {
    fn to_node_id(&self, task: &TaskHandle) -> NodeId {
        (*self).to_node_id(task)
    }
}

/// A handle to spawn tasks on a node.
#[derive(Clone)]
pub struct Spawner {
    sender: mpsc::Sender<(Runnable, Arc<TaskInfo>)>,
    info: Arc<NodeInfo>,
}

/// A handle to spawn tasks on a node.
#[deprecated(since = "0.3", note = "use Spawner instead")]
pub type TaskNodeHandle = Spawner;

impl Spawner {
    fn current() -> Self {
        let info = crate::context::current_task();
        let sender = crate::context::current(|h| h.task.sender.clone());
        Spawner {
            sender,
            info: info.node.clone(),
        }
    }

    pub(crate) fn node_id(&self) -> NodeId {
        self.info.id
    }

    /// Spawns a new asynchronous task, returning a [`JoinHandle`] for it.
    pub fn spawn<F>(&self, future: F) -> JoinHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        self.spawn_inner(future, None)
    }

    /// Spawns a `!Send` future on the local task set.
    pub fn spawn_local<F>(&self, future: F) -> JoinHandle<F::Output>
    where
        F: Future + 'static,
        F::Output: 'static,
    {
        self.spawn_inner(future, None)
    }

    /// Spawns a future on with name.
    fn spawn_inner<F>(&self, future: F, name: Option<&str>) -> JoinHandle<F::Output>
    where
        F: Future + 'static,
        F::Output: 'static,
    {
        let sender = self.sender.clone();
        let info = self.info.new_task(name);
        let id = info.id;
        trace!(%id, name, "spawn task");

        let (runnable, task) = unsafe {
            // Safety: The schedule is not Sync,
            // the task's Waker must be used and dropped on the original thread.
            async_task::spawn_unchecked(future, move |runnable| {
                trace!(%id, name, "wake task");
                let _ = sender.send((runnable, info.clone()));
            })
        };
        runnable.schedule();

        JoinHandle {
            id,
            task: Mutex::new(Some(task.fallible())),
        }
    }
}

/// Spawns a new asynchronous task, returning a [`JoinHandle`] for it.
#[track_caller]
pub fn spawn<F>(future: F) -> JoinHandle<F::Output>
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    Spawner::current().spawn(future)
}

/// Spawns a `!Send` future on the local task set.
#[track_caller]
pub fn spawn_local<F>(future: F) -> JoinHandle<F::Output>
where
    F: Future + 'static,
    F::Output: 'static,
{
    Spawner::current().spawn_local(future)
}

/// Runs the provided closure on a thread where blocking is acceptable.
#[deprecated(since = "0.3", note = "blocking function is not allowed in simulation")]
#[track_caller]
pub fn spawn_blocking<F, R>(f: F) -> JoinHandle<R>
where
    F: FnOnce() -> R + Send + 'static,
    R: Send + 'static,
{
    Spawner::current().spawn(async move { f() })
}

/// Factory which is used to configure the properties of a new task.
#[derive(Default, Debug)]
pub struct Builder<'a> {
    name: Option<&'a str>,
}

impl<'a> Builder<'a> {
    /// Creates a new task builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Assigns a name to the task which will be spawned.
    pub fn name(&self, name: &'a str) -> Self {
        Self { name: Some(name) }
    }

    /// Spawns a task with this builder's settings on the current runtime.
    #[track_caller]
    pub fn spawn<Fut>(self, future: Fut) -> JoinHandle<Fut::Output>
    where
        Fut: Future + Send + 'static,
        Fut::Output: Send + 'static,
    {
        Spawner::current().spawn_inner(future, self.name)
    }

    /// Spawns `!Send` a task on the current [`LocalSet`] with this builder's settings.
    #[track_caller]
    pub fn spawn_local<Fut>(self, future: Fut) -> JoinHandle<Fut::Output>
    where
        Fut: Future + 'static,
        Fut::Output: 'static,
    {
        Spawner::current().spawn_inner(future, self.name)
    }
}

/// An opaque ID that uniquely identifies a task relative to all other currently running tasks.
#[derive(Clone, Copy, Debug, Hash, Eq, PartialEq)]
pub struct Id(u64);

impl Id {
    fn new() -> Self {
        static NEXT_TASK_ID: AtomicU64 = AtomicU64::new(0);
        Id(NEXT_TASK_ID.fetch_add(1, Ordering::SeqCst))
    }
}

impl fmt::Display for Id {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// An owned permission to join on a task (await its termination).
#[derive(Debug)]
pub struct JoinHandle<T> {
    id: Id,
    task: Mutex<Option<FallibleTask<T>>>,
}

impl<T> JoinHandle<T> {
    /// Abort the task associated with the handle.
    pub fn abort(&self) {
        self.task.lock().take();
    }

    /// Cancel the task when this handle is dropped.
    #[doc(hidden)]
    pub fn cancel_on_drop(self) -> FallibleTask<T> {
        self.task.lock().take().unwrap()
    }
}

impl<T> Future for JoinHandle<T> {
    type Output = Result<T, JoinError>;

    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        std::pin::Pin::new(self.task.lock().as_mut().unwrap())
            .poll(cx)
            .map(|res| {
                res.ok_or(JoinError {
                    id: self.id,
                    is_panic: true, // TODO: decide cancelled or panic
                })
            })
    }
}

impl<T> Drop for JoinHandle<T> {
    fn drop(&mut self) {
        if let Some(task) = self.task.lock().take() {
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

/// For `std::thread::available_parallelism` on Linux.
///
/// Ref: <https://man7.org/linux/man-pages/man2/sched_setaffinity.2.html>
#[no_mangle]
#[inline(never)]
#[cfg(target_os = "linux")]
unsafe extern "C" fn sched_getaffinity(
    pid: libc::pid_t,
    cpusetsize: libc::size_t,
    cpuset: *mut libc::cpu_set_t,
) -> libc::c_int {
    if let Some(info) = crate::context::try_current_task() {
        assert_eq!(cpusetsize, std::mem::size_of::<libc::cpu_set_t>());
        assert!(cpusetsize * 8 >= info.node.cores as _);
        for i in 0..info.node.cores {
            libc::CPU_SET(i, &mut *cpuset);
        }
        return 0;
    }
    lazy_static::lazy_static! {
        static ref SCHED_GETAFFINITY: unsafe extern "C" fn(
            pid: libc::pid_t,
            cpusetsize: libc::size_t,
            cpuset: *mut libc::cpu_set_t,
        ) -> libc::c_int = unsafe {
            let ptr = libc::dlsym(libc::RTLD_NEXT, b"sched_getaffinity\0".as_ptr() as _);
            assert!(!ptr.is_null());
            std::mem::transmute(ptr)
        };
    }
    SCHED_GETAFFINITY(pid, cpusetsize, cpuset)
}

/// For `std::thread::available_parallelism` on macOS.
///
/// Ref: <https://man7.org/linux/man-pages/man3/sysconf.3.html>
#[no_mangle]
#[inline(never)]
unsafe extern "C" fn sysconf(name: libc::c_int) -> libc::c_long {
    if name == libc::_SC_NPROCESSORS_ONLN {
        if let Some(info) = crate::context::try_current_task() {
            return info.node.cores as _;
        }
    }
    lazy_static::lazy_static! {
        static ref SYSCONF: unsafe extern "C" fn(name: libc::c_int) -> libc::c_long = unsafe {
            let ptr = libc::dlsym(libc::RTLD_NEXT, b"sysconf\0".as_ptr() as _);
            assert!(!ptr.is_null());
            std::mem::transmute(ptr)
        };
    }
    SYSCONF(name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        runtime::{Handle, Runtime},
        time,
    };
    use std::{collections::HashSet, sync::atomic::AtomicUsize, time::Duration};

    #[test]
    fn spawn_in_block_on() {
        let runtime = Runtime::new();
        runtime.block_on(async {
            spawn(async { 1 }).await.unwrap();
            spawn_local(async { 2 }).await.unwrap();
        });
    }

    #[test]
    fn kill() {
        let runtime = Runtime::new();
        let node1 = runtime.create_node().build();
        let node2 = runtime.create_node().build();

        let flag1 = Arc::new(AtomicUsize::new(0));
        let flag2 = Arc::new(AtomicUsize::new(0));

        let flag1_ = flag1.clone();
        node1.spawn(async move {
            loop {
                time::sleep(Duration::from_secs(2)).await;
                flag1_.fetch_add(2, Ordering::SeqCst);
            }
        });

        let flag2_ = flag2.clone();
        node2.spawn(async move {
            loop {
                time::sleep(Duration::from_secs(2)).await;
                flag2_.fetch_add(2, Ordering::SeqCst);
            }
        });

        runtime.block_on(async move {
            let t0 = time::Instant::now();

            time::sleep_until(t0 + Duration::from_secs(3)).await;
            assert_eq!(flag1.load(Ordering::SeqCst), 2);
            assert_eq!(flag2.load(Ordering::SeqCst), 2);
            Handle::current().kill(node1.id());
            Handle::current().kill(node1.id());

            time::sleep_until(t0 + Duration::from_secs(5)).await;
            assert_eq!(flag1.load(Ordering::SeqCst), 2);
            assert_eq!(flag2.load(Ordering::SeqCst), 4);
        });
    }

    #[test]
    fn restart() {
        let runtime = Runtime::new();

        let flag = Arc::new(AtomicUsize::new(0));

        let flag_ = flag.clone();
        let node = runtime
            .create_node()
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
            .build();

        runtime.block_on(async move {
            let t0 = time::Instant::now();

            time::sleep_until(t0 + Duration::from_secs(3)).await;
            assert_eq!(flag.load(Ordering::SeqCst), 2);
            Handle::current().kill(node.id());
            Handle::current().restart(node.id());

            time::sleep_until(t0 + Duration::from_secs(6)).await;
            assert_eq!(flag.load(Ordering::SeqCst), 2);

            time::sleep_until(t0 + Duration::from_secs(8)).await;
            assert_eq!(flag.load(Ordering::SeqCst), 4);
        });
    }

    #[test]
    fn pause_resume() {
        let runtime = Runtime::new();
        let node = runtime.create_node().build();

        let flag = Arc::new(AtomicUsize::new(0));
        let flag_ = flag.clone();
        node.spawn(async move {
            loop {
                time::sleep(Duration::from_secs(2)).await;
                flag_.fetch_add(2, Ordering::SeqCst);
            }
        });

        runtime.block_on(async move {
            let t0 = time::Instant::now();

            time::sleep_until(t0 + Duration::from_secs(3)).await;
            assert_eq!(flag.load(Ordering::SeqCst), 2);
            Handle::current().pause(node.id());
            Handle::current().pause(node.id());

            time::sleep_until(t0 + Duration::from_secs(5)).await;
            assert_eq!(flag.load(Ordering::SeqCst), 2);

            Handle::current().resume(node.id());
            Handle::current().resume(node.id());
            time::sleep_until(t0 + Duration::from_secs_f32(5.5)).await;
            assert_eq!(flag.load(Ordering::SeqCst), 4);
        });
    }

    #[test]
    fn random_select_from_ready_tasks() {
        let mut seqs = HashSet::new();
        for seed in 0..10 {
            let runtime = Runtime::with_seed_and_config(seed, crate::Config::default());
            let seq = runtime.block_on(async {
                let (tx, rx) = std::sync::mpsc::channel();
                let mut tasks = vec![];
                for i in 0..3 {
                    let tx = tx.clone();
                    tasks.push(spawn(async move {
                        for j in 0..5 {
                            tx.send(i * 10 + j).unwrap();
                            tokio::task::yield_now().await;
                        }
                    }));
                }
                drop(tx);
                futures_util::future::join_all(tasks).await;
                rx.into_iter().collect::<Vec<_>>()
            });
            seqs.insert(seq);
        }
        assert_eq!(seqs.len(), 10);
    }

    #[test]
    fn deterministic_std_thread_available_parallelism() {
        let runtime = Runtime::new();
        runtime.block_on(async move {
            // available_parallelism is 1 by default
            assert_eq!(std::thread::available_parallelism().unwrap().get(), 1);
        });
        let f1 = runtime.create_node().build().spawn(async move {
            // available_parallelism is 1 by default
            assert_eq!(std::thread::available_parallelism().unwrap().get(), 1);
        });
        let f2 = runtime.create_node().cores(128).build().spawn(async move {
            assert_eq!(std::thread::available_parallelism().unwrap().get(), 128);
        });
        runtime.block_on(f1).unwrap();
        runtime.block_on(f2).unwrap();
    }
}
