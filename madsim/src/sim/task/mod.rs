//! Asynchronous tasks executor.

use super::{
    rand::GlobalRng,
    runtime::{NodeBuilder, Simulators},
    time::{TimeHandle, TimeRuntime},
    utils::mpsc,
};
use futures_util::FutureExt;
use rand::Rng;
use spin::Mutex;
use std::{
    collections::{BTreeMap, HashMap},
    fmt,
    future::Future,
    io,
    ops::Deref,
    panic::Location,
    pin::Pin,
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc, Weak,
    },
    task::{Context, Poll, Waker},
    time::{Duration, Instant},
};
use tokio::sync::watch;
use tracing::{debug, error, error_span, trace, Span};

pub use tokio::task::yield_now;

type StaticLocation = &'static Location<'static>;
type Runnable = async_task::Runnable<Weak<TaskInfo>>;
#[doc(hidden)]
pub type FallibleTask<T> = async_task::FallibleTask<T, Weak<TaskInfo>>;

mod builder;
mod join;

pub use self::builder::*;
pub use self::join::*;

pub(crate) struct Executor {
    queue: mpsc::Receiver<Runnable>,
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

// The lifetime of `TaskInfo` equals to the future.
#[doc(hidden)]
pub struct TaskInfo {
    pub id: Id,
    pub name: Option<String>,
    pub(crate) node: Arc<NodeInfo>,
    /// The span of this task.
    pub span: Span,
    /// The spawn location.
    location: StaticLocation,
    /// The time when this task is spawned.
    #[allow(dead_code)]
    spawn_time: Instant,
    /// The waker of this task. Used for cancellation.
    waker: Waker,
    /// A flag indicating that the task has been cancelled.
    cancelled: AtomicBool,
}

pub(crate) struct NodeInfo {
    pub id: NodeId,
    /// Node name.
    name: Option<String>,
    /// The number of CPU cores.
    cores: usize,
    /// Whether to restart the node on panic.
    restart_on_panic: bool,
    /// The list of panic messages that will cause the node to restart.
    restart_on_panic_matching: Vec<String>,
    /// The span of this node.
    span: Span,

    /// A flag indicating that the node has been paused.
    paused: AtomicBool,
    /// A flag indicating that the node has been killed.
    killed: AtomicBool,
    /// All tasks spawned in this node.
    tasks: Mutex<Vec<Weak<TaskInfo>>>,
    /// Sender of the "ctrl-c" signal.
    ///
    /// This value is `None` at the beginning, meaning that `signal::ctrl_c` has never been called,
    /// and sending "ctrl-c" will cause the node being killed. Once `signal::ctrl_c` is called,
    /// this will be set to `Some`, and sending "ctrl-c" will no longer kill the node.
    ctrl_c: Mutex<Option<watch::Sender<()>>>,
}

impl NodeInfo {
    #[track_caller]
    fn new_task(self: &Arc<Self>, name: Option<&str>) -> Arc<TaskInfo> {
        let id = Id::new();
        let name = name.map(|s| s.to_string());
        let task = Arc::new(TaskInfo {
            span: error_span!(parent: &self.span, "task", %id, name),
            id,
            name,
            node: self.clone(),
            location: Location::caller(),
            spawn_time: Instant::now(),
            waker: futures_util::task::noop_waker(), // updated later
            cancelled: AtomicBool::new(false),
        });
        self.tasks.lock().push(Arc::downgrade(&task));
        task
    }

    fn kill(&self) {
        self.killed.store(true, Ordering::Relaxed);
        for task in self.tasks.lock().drain(..) {
            if let Some(task) = task.upgrade() {
                task.waker.wake_by_ref();
            }
        }
    }

    fn num_tasks(&self) -> usize {
        let mut tasks = self.tasks.lock();
        tasks.retain(|weak| weak.strong_count() != 0);
        tasks.len()
    }

    fn num_tasks_by_spawn(&self) -> BTreeMap<String, usize> {
        let mut tasks = self.tasks.lock();
        let mut map = BTreeMap::new();
        tasks.retain(|weak| {
            if let Some(task) = weak.upgrade() {
                *map.entry(task.location.to_string()).or_default() += 1;
                true
            } else {
                false
            }
        });
        map
    }

    pub(crate) fn is_killed(&self) -> bool {
        self.killed.load(Ordering::Relaxed)
    }

    /// Get a receiver of "ctrl-c" signal.
    pub(crate) fn ctrl_c(&self) -> watch::Receiver<()> {
        self.ctrl_c
            .lock()
            .get_or_insert_with(|| {
                debug!("ctrl-c signal handler installed");
                watch::channel(()).0
            })
            .subscribe()
    }
}

impl Executor {
    pub fn new(rand: GlobalRng, sims: Arc<Simulators>) -> Self {
        let (sender, queue) = mpsc::channel();
        Executor {
            queue,
            handle: TaskHandle {
                nodes: Arc::new(Mutex::new(HashMap::new())),
                sender,
                next_node_id: Arc::new(AtomicU64::new(1)),
                main_info: Arc::new(NodeInfo {
                    id: NodeId::zero(),
                    name: Some("main".into()),
                    cores: 1,
                    restart_on_panic: false,
                    restart_on_panic_matching: vec![],
                    span: error_span!("node", id = %NodeId::zero(), name = "main"),
                    paused: AtomicBool::new(false),
                    killed: AtomicBool::new(false),
                    tasks: Mutex::new(vec![]),
                    ctrl_c: Mutex::new(None),
                }),
                sims,
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

    #[track_caller]
    pub fn block_on<F: Future>(&self, future: F) -> F::Output {
        // push the future into ready queue.
        let sender = self.handle.sender.clone();
        let info = self.handle.main_info.new_task(None);
        let (runnable, task) = unsafe {
            async_task::Builder::new()
                .metadata(Arc::downgrade(&info))
                .spawn_unchecked(
                    move |_| async move {
                        let _info = info; // drop the info when the future is dropped
                        future.await
                    },
                    move |runnable| _ = sender.send(runnable),
                )
        };
        runnable.schedule();

        let allow_system_thread =
            crate::context::try_current(|h| h.allow_system_thread).unwrap_or_default();
        loop {
            self.run_all_ready();
            if task.is_finished() {
                return task.now_or_never().unwrap();
            }
            let going = self.time.advance_to_next_event();
            if !going {
                if allow_system_thread {
                    // other system threads may wake up the tasks, wait for a while
                    std::thread::sleep(std::time::Duration::from_millis(1));
                } else {
                    panic!("no events, all tasks will block forever");
                }
            }
            if let Some(limit) = self.time_limit {
                assert!(
                    self.time.handle().elapsed() < limit,
                    "time limit exceeded: {limit:?}"
                )
            }
        }
    }

    /// Drain all tasks from ready queue and run them.
    fn run_all_ready(&self) {
        while let Ok(runnable) = self.queue.try_recv_random(&self.rand) {
            let Some(info) = runnable.metadata().upgrade() else {
                // future has been dropped
                continue;
            };
            let work = if info.cancelled.load(Ordering::Relaxed)
                || info.node.killed.load(Ordering::Relaxed)
            {
                // cancelled task or killed node: drop the future
                drop
            } else if info.node.paused.load(Ordering::Relaxed) {
                // paused task: push to waiting list
                (self.nodes.lock().get_mut(&info.node.id).unwrap().paused).push(runnable);
                continue;
            } else {
                fn run(runnable: Runnable) {
                    runnable.run();
                }
                run
            };
            // run the task
            let res = {
                let _guard = crate::context::enter_task(info.clone());
                std::panic::catch_unwind(move || work(runnable))
            };
            if let Err(e) = res {
                eprintln!(
                    "context: node={} {:?}, task={} (spawned at {})",
                    info.node.id,
                    info.node.name.as_ref().map_or("<unnamed>", |s| s),
                    info.id,
                    info.location
                );
                let error_msg = panic_message::panic_message(&e);
                if info.node.restart_on_panic
                    || (info.node.restart_on_panic_matching.iter()).any(|s| error_msg.contains(s))
                {
                    let node_id = info.node.id;
                    let delay = self
                        .rand
                        .with(|rng| rng.gen_range(Duration::from_secs(1)..Duration::from_secs(10)));
                    error!(
                        "task panicked, restarting node {} {:?} after {:?}",
                        node_id, info.node.name, delay
                    );
                    self.kill(node_id);
                    let h = self.handle.clone();
                    self.time
                        .handle()
                        .add_timer(delay, move || h.restart(node_id));
                } else {
                    std::panic::resume_unwind(e);
                }
            }

            // advance time: 50-100ns
            let dur = Duration::from_nanos(self.rand.with(|rng| rng.gen_range(50..100)));
            self.time.handle().advance(dur);
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
    pub(crate) sender: mpsc::Sender<Runnable>,
    nodes: Arc<Mutex<HashMap<NodeId, Node>>>,
    next_node_id: Arc<AtomicU64>,
    /// Info of the main node.
    main_info: Arc<NodeInfo>,
    sims: Arc<Simulators>,
}

struct Node {
    info: Arc<NodeInfo>,
    paused: Vec<Runnable>,
    /// A function to spawn the initial task.
    init: Option<InitFn>,
}

pub(crate) type InitFn = Arc<dyn Fn(&Spawner) + Send + Sync>;

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
        node.info.kill();

        for sim in self.sims.lock().values() {
            sim.reset_node(id);
        }
    }

    /// Kill all tasks of the node and restart the initial task.
    pub fn restart(&self, id: impl ToNodeId) {
        debug!(node = %id, "restart");
        let id = id.to_node_id(self);
        let mut nodes = self.nodes.lock();
        let node = nodes.get_mut(&id).expect("node not found");
        let new_info = Arc::new(NodeInfo {
            id,
            name: node.info.name.clone(),
            cores: node.info.cores,
            restart_on_panic: node.info.restart_on_panic,
            restart_on_panic_matching: node.info.restart_on_panic_matching.clone(),
            paused: AtomicBool::new(false),
            killed: AtomicBool::new(false),
            span: error_span!(parent: None, "node", %id, name = &node.info.name),
            tasks: Mutex::new(vec![]),
            ctrl_c: Mutex::new(None),
        });
        let old_info = std::mem::replace(&mut node.info, new_info);
        node.paused.clear();
        old_info.kill();

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
        node.info.paused.store(true, Ordering::Relaxed);
    }

    /// Resume the execution of the address.
    pub fn resume(&self, id: impl ToNodeId) {
        debug!(node = %id, "resume");
        let id = id.to_node_id(self);
        let mut nodes = self.nodes.lock();
        let node = nodes.get_mut(&id).expect("node not found");
        node.info.paused.store(false, Ordering::Relaxed);

        // take paused tasks from waiting list and push them to ready queue
        for runnable in node.paused.drain(..) {
            self.sender.send(runnable).unwrap();
        }
    }

    /// Send a "ctrl-c" signal to the node.
    pub fn send_ctrl_c(&self, id: impl ToNodeId) {
        debug!(node = %id, "send ctrl-c");
        let id = id.to_node_id(self);
        let mut nodes = self.nodes.lock();
        let node = nodes.get_mut(&id).expect("node not found");
        if let Some(tx) = &*node.info.ctrl_c.lock() {
            // may return error if no receiver
            _ = tx.send(());
            return;
        }
        drop(nodes);
        // "ctrl-c" has never been called. kill node
        debug!(node = %id, "killed by ctrl-c");
        self.kill_id(id);
    }

    /// Returns whether the node is killed or exited.
    pub fn is_exit(&self, id: impl ToNodeId) -> bool {
        let id = id.to_node_id(self);
        let nodes = self.nodes.lock();
        let node = nodes.get(&id).expect("node not found");
        node.info.killed.load(Ordering::Relaxed)
    }

    /// Create a new node.
    pub fn create_node(&self, builder: &NodeBuilder<'_>) -> Spawner {
        let id = NodeId(self.next_node_id.fetch_add(1, Ordering::Relaxed));
        let name = &builder.name;
        debug!(node = %id, name, "create");
        let info = Arc::new(NodeInfo {
            span: error_span!(parent: None, "node", %id, name),
            id,
            name: builder.name.clone(),
            cores: builder.cores.unwrap_or(1),
            restart_on_panic: builder.restart_on_panic,
            restart_on_panic_matching: builder.restart_on_panic_matching.clone(),
            paused: AtomicBool::new(false),
            killed: AtomicBool::new(false),
            tasks: Mutex::new(vec![]),
            ctrl_c: Mutex::new(None),
        });
        let handle = Spawner {
            sender: self.sender.clone(),
            info: info.clone(),
        };
        if let Some(init) = &builder.init {
            init(&handle);
        }
        let node = Node {
            info,
            paused: vec![],
            init: builder.init.clone(),
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

    pub fn num_nodes(&self) -> usize {
        self.nodes.lock().len()
    }

    pub fn num_tasks(&self) -> usize {
        self.nodes
            .lock()
            .values()
            .map(|node| node.info.num_tasks())
            .sum()
    }

    pub fn num_tasks_by_node(&self) -> BTreeMap<String, usize> {
        self.nodes
            .lock()
            .values()
            .map(|node| {
                (
                    node.info
                        .name
                        .clone()
                        .unwrap_or_else(|| format!("node-{}", node.info.id)),
                    node.info.num_tasks(),
                )
            })
            .collect()
    }

    pub fn num_tasks_by_node_by_spawn(&self) -> String {
        let map = self
            .nodes
            .lock()
            .values()
            .map(|node| {
                (
                    node.info
                        .name
                        .clone()
                        .unwrap_or_else(|| format!("node-{}", node.info.id)),
                    node.info.num_tasks_by_spawn(),
                )
            })
            .collect::<BTreeMap<String, _>>();
        format!("{map:#?}")
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
        match (task.nodes.lock().iter()).find(|(_, node)| match &node.info.name {
            Some(name) => name == self,
            None => false,
        }) {
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
    pub(crate) sender: mpsc::Sender<Runnable>,
    pub(crate) info: Arc<NodeInfo>,
}

/// A handle to spawn tasks on a node.
#[deprecated(since = "0.3.0", note = "use Spawner instead")]
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
    #[track_caller]
    pub fn spawn<F>(&self, future: F) -> JoinHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        self.spawn_inner(future, None)
    }

    /// Spawns a `!Send` future on the local task set.
    #[track_caller]
    pub fn spawn_local<F>(&self, future: F) -> JoinHandle<F::Output>
    where
        F: Future + 'static,
        F::Output: 'static,
    {
        self.spawn_inner(future, None)
    }

    /// Spawns a future on with name.
    #[track_caller]
    fn spawn_inner<F>(&self, future: F, name: Option<&str>) -> JoinHandle<F::Output>
    where
        F: Future + 'static,
        F::Output: 'static,
    {
        if self.info.killed.load(Ordering::Relaxed) {
            tracing::warn!("spawning task on a killed node, the task will never run");
        }
        let sender = self.sender.clone();
        let info = self.info.new_task(name);
        trace!(id = %info.id, name, "spawn task");

        let info1 = info.clone();
        let (runnable, task) = async_task::Builder::new()
            .metadata(Arc::downgrade(&info))
            .spawn_local(
                move |_| async move {
                    let _info = info1; // drop the info when the future is dropped
                    future.await
                },
                move |runnable| _ = sender.send(runnable),
            );
        // SAFETY: info can not be accessed by others.
        unsafe { &mut *Arc::as_ptr(&info).cast_mut() }.waker = runnable.waker();
        runnable.schedule();

        JoinHandle::new(info, task.fallible())
    }

    /// Exit the current process (node).
    pub(crate) fn exit(&self) {
        debug!(node = %self.info.id, "exit");
        // FIXME: clear paused tasks
        self.info.kill();
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
#[deprecated(
    since = "0.3.0",
    note = "blocking function is not allowed in simulation"
)]
pub fn spawn_blocking<F, R>(f: F) -> JoinHandle<R>
where
    F: FnOnce() -> R + Send + 'static,
    R: Send + 'static,
{
    Spawner::current().spawn(async move { f() })
}

/// An opaque ID that uniquely identifies a task relative to all other currently running tasks.
#[derive(Clone, Copy, Debug, Hash, Eq, PartialEq)]
pub struct Id(u64);

impl Id {
    fn new() -> Self {
        static NEXT_TASK_ID: AtomicU64 = AtomicU64::new(0);
        Id(NEXT_TASK_ID.fetch_add(1, Ordering::Relaxed))
    }
}

impl fmt::Display for Id {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
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
            let ptr = libc::dlsym(libc::RTLD_NEXT, c"sched_getaffinity".as_ptr() as _);
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
            let ptr = libc::dlsym(libc::RTLD_NEXT, c"sysconf".as_ptr() as _);
            assert!(!ptr.is_null());
            std::mem::transmute(ptr)
        };
    }
    SYSCONF(name)
}

/// Forbid creating system thread in simulation.
#[no_mangle]
#[inline(never)]
unsafe extern "C" fn pthread_attr_init(attr: *mut libc::pthread_attr_t) -> libc::c_int {
    if let Some(allowed) = crate::context::try_current(|h| h.allow_system_thread) {
        if allowed {
            tracing::warn!(
                "spawning a system thread in simulation. this may cause non-determinism."
            );
        } else {
            eprintln!("attempt to spawn a system thread in simulation.");
            eprintln!("note: try to use tokio tasks instead.");
            eprintln!("note: if you really need to spawn a system thread, set the environment variable `MADSIM_ALLOW_SYSTEM_THREAD` to `1`.");
            return -1;
        }
    }
    lazy_static::lazy_static! {
        static ref PTHREAD_ATTR_INIT: unsafe extern "C" fn(attr: *mut libc::pthread_attr_t) -> libc::c_int = unsafe {
            let ptr = libc::dlsym(libc::RTLD_NEXT, c"pthread_attr_init".as_ptr() as _);
            assert!(!ptr.is_null());
            std::mem::transmute(ptr)
        };
    }
    PTHREAD_ATTR_INIT(attr)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        context::{current, current_node},
        runtime::{init_logger, Handle, Runtime},
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
                flag1_.fetch_add(2, Ordering::Relaxed);
            }
        });

        let flag2_ = flag2.clone();
        node2.spawn(async move {
            loop {
                time::sleep(Duration::from_secs(2)).await;
                flag2_.fetch_add(2, Ordering::Relaxed);
            }
        });

        runtime.block_on(async move {
            let t0 = time::Instant::now();

            time::sleep_until(t0 + Duration::from_secs(3)).await;
            assert_eq!(flag1.load(Ordering::Relaxed), 2);
            assert_eq!(flag2.load(Ordering::Relaxed), 2);
            Handle::current().kill(node1.id());
            Handle::current().kill(node1.id());
            assert!(Handle::current().is_exit(node1.id()));

            time::sleep_until(t0 + Duration::from_secs(5)).await;
            assert_eq!(flag1.load(Ordering::Relaxed), 2);
            assert_eq!(flag2.load(Ordering::Relaxed), 4);
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
                    flag.store(0, Ordering::Relaxed);
                    loop {
                        time::sleep(Duration::from_secs(2)).await;
                        flag.fetch_add(2, Ordering::Relaxed);
                    }
                }
            })
            .build();

        runtime.block_on(async move {
            let t0 = time::Instant::now();

            time::sleep_until(t0 + Duration::from_secs(3)).await;
            assert_eq!(flag.load(Ordering::Relaxed), 2);
            Handle::current().kill(node.id());
            Handle::current().restart(node.id());
            assert!(!Handle::current().is_exit(node.id()));

            time::sleep_until(t0 + Duration::from_secs(6)).await;
            assert_eq!(flag.load(Ordering::Relaxed), 2);

            time::sleep_until(t0 + Duration::from_secs(8)).await;
            assert_eq!(flag.load(Ordering::Relaxed), 4);
        });
    }

    #[test]
    fn restart_on_panic() {
        let runtime = Runtime::new();
        let flag = Arc::new(AtomicUsize::new(0));

        let flag_ = flag.clone();
        runtime
            .create_node()
            .init(move || {
                let flag = flag_.clone();
                async move {
                    if flag.fetch_add(1, Ordering::Relaxed) < 3 {
                        panic!();
                    }
                }
            })
            .restart_on_panic()
            .build();

        runtime.block_on(async move {
            time::sleep(Duration::from_secs(60)).await;
            // should panic 3 times and success once
            assert_eq!(flag.load(Ordering::Relaxed), 4);
        });
    }

    #[test]
    #[should_panic(expected = "2")]
    fn restart_on_panic_matching() {
        let runtime = Runtime::new();
        let flag = Arc::new(AtomicUsize::new(0));

        runtime
            .create_node()
            .init(move || {
                let flag = flag.clone();
                async move {
                    panic!("{}", flag.fetch_add(1, Ordering::Relaxed));
                }
            })
            .restart_on_panic_matching("0")
            .restart_on_panic_matching("1")
            .build();

        runtime.block_on(std::future::pending::<()>());
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
                flag_.fetch_add(2, Ordering::Relaxed);
            }
        });

        runtime.block_on(async move {
            let t0 = time::Instant::now();

            time::sleep_until(t0 + Duration::from_secs(3)).await;
            assert_eq!(flag.load(Ordering::Relaxed), 2);
            Handle::current().pause(node.id());
            Handle::current().pause(node.id());

            time::sleep_until(t0 + Duration::from_secs(5)).await;
            assert_eq!(flag.load(Ordering::Relaxed), 2);

            Handle::current().resume(node.id());
            Handle::current().resume(node.id());
            time::sleep_until(t0 + Duration::from_secs_f32(5.5)).await;
            assert_eq!(flag.load(Ordering::Relaxed), 4);
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

    #[test]
    #[should_panic]
    fn forbid_creating_system_thread() {
        let runtime = Runtime::new();
        runtime.block_on(async move {
            std::thread::spawn(|| {});
        });
    }

    #[test]
    fn allow_creating_system_thread() {
        let mut runtime = Runtime::new();
        runtime.set_allow_system_thread(true);
        runtime.block_on(async move {
            let (send, recv) = tokio::sync::oneshot::channel();
            std::thread::spawn(move || {
                std::thread::sleep(Duration::from_millis(100));
                send.send(()).unwrap();
            });
            recv.await.unwrap();
        });
    }

    #[test]
    fn kill_drop_futures() {
        let runtime = Runtime::new();
        let node = runtime.create_node().build();

        let flag = Arc::new(());
        let flag_ = flag.clone();
        node.spawn(async move {
            std::future::pending::<()>().await;
            drop(flag_);
        });

        runtime.block_on(async move {
            time::sleep(Duration::from_secs(1)).await;
            // make sure the future is pending

            Handle::current().kill(node.id());
            assert_eq!(Arc::strong_count(&flag), 2);

            // future will be dropped after a while
            time::sleep(Duration::from_secs(1)).await;
            assert_eq!(Arc::strong_count(&flag), 1);
        });
    }

    #[test]
    fn join_cancelled() {
        let runtime = Runtime::new();
        let node = runtime.create_node().build();

        let handle = node.spawn(async move {
            std::future::pending::<()>().await;
        });

        runtime.block_on(async move {
            handle.abort();
            let err = handle.await.unwrap_err();
            assert!(err.is_cancelled());
        });
    }

    #[test]
    fn exited() {
        let runtime = Runtime::new();

        let flag = Arc::new(AtomicUsize::new(0));
        let flag_ = flag.clone();
        let node = runtime
            .create_node()
            .init(move || {
                let flag_ = flag_.clone();
                async move {
                    crate::task::spawn(async move {
                        loop {
                            time::sleep(Duration::from_secs(2)).await;
                            flag_.fetch_add(2, Ordering::Relaxed);
                        }
                    });
                    time::sleep(Duration::from_secs(5)).await;
                    // exit here. the spawned task will be killed
                }
            })
            .build();

        runtime.block_on(async move {
            assert!(!Handle::current().is_exit(node.id()));
            time::sleep(Duration::from_secs(10)).await;
            assert!(Handle::current().is_exit(node.id()));
            assert_eq!(flag.load(Ordering::Relaxed), 4);
        });
    }

    #[test]
    fn spawn_in_future_drop_by_aborting_task() {
        static DROPPING_NODE: Mutex<Option<NodeId>> = Mutex::new(None);

        struct A;

        impl Drop for A {
            fn drop(&mut self) {
                spawn(async move {
                    let _ = DROPPING_NODE.try_lock().unwrap().insert(current_node());
                });
            }
        }

        let runtime = Runtime::new();
        let node = runtime.create_node().build();
        let node_id = node.id();
        let a = A;

        runtime.block_on(async move {
            // Since we're a single-threaded runtime, task spawning here won't run proactively.
            // So `drop` will only be called after we call `abort` and while `await`ing the handle.
            let join_handle = node.spawn(async move { drop(a) });
            join_handle.abort();

            let err = join_handle.await.unwrap_err();
            assert!(err.is_cancelled());

            // Let the task spawned in `A::drop` run and set the node id.
            time::sleep(Duration::from_secs(114514)).await;
            // Check that the task spawned in `A::drop` is also under the `node` we created.
            assert_eq!(DROPPING_NODE.try_lock().unwrap().unwrap(), node_id);
        })
    }

    #[test]
    fn spawn_in_future_drop_by_killing_node() {
        init_logger();

        static DROPPED: Mutex<bool> = Mutex::new(false);

        struct A;

        impl Drop for A {
            fn drop(&mut self) {
                // Spawning a task on a killed node will still succeed, but the task will never run.
                spawn(async move { unreachable!() });
                *DROPPED.try_lock().unwrap() = true;
            }
        }

        let runtime = Runtime::new();
        let node = runtime.create_node().build();
        let node_id = node.id();
        let a = A;

        runtime.block_on(async move {
            // Since we're a single-threaded runtime, task spawning here won't run proactively.
            // So `drop` will only be called after we call `kill` and while `await`ing the handle.
            let join_handle = node.spawn(async move { drop(a) });
            current(|handle| handle.kill(node_id));

            let err = join_handle.await.unwrap_err();
            assert!(err.is_cancelled());
            assert!(*DROPPED.try_lock().unwrap());

            // Give some time, showing that the task spawned in `A::drop` is never run.
            time::sleep(Duration::from_secs(114514)).await;
        })
    }
}
