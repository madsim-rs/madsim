//! Thread local runtime context
use crate::{task::TaskInfo, Handle};

use std::{cell::RefCell, sync::Arc};

thread_local! {
    static CONTEXT: RefCell<Option<Handle>> = RefCell::new(None);
    static TASK: RefCell<Option<Arc<TaskInfo>>> = RefCell::new(None);
}

pub(crate) fn current() -> Handle {
    CONTEXT.with(|ctx| ctx.borrow().clone().expect(MSG))
}

#[allow(dead_code)]
pub(crate) fn current_task() -> Option<Arc<TaskInfo>> {
    TASK.with(|task| task.borrow().clone())
}

pub(crate) fn rand_handle() -> crate::rand::RandHandle {
    CONTEXT.with(|ctx| ctx.borrow().as_ref().expect(MSG).rand.clone())
}

pub(crate) fn time_handle() -> crate::time::TimeHandle {
    CONTEXT.with(|ctx| ctx.borrow().as_ref().expect(MSG).time.clone())
}

pub(crate) fn try_time_handle() -> Option<crate::time::TimeHandle> {
    CONTEXT.with(|ctx| ctx.borrow().as_ref().map(|h| h.time.clone()))
}

pub(crate) fn task_local_handle() -> crate::task::TaskLocalHandle {
    let addr = TASK.with(|task| task.borrow().as_ref().expect(MSG).addr);
    CONTEXT.with(|ctx| {
        ctx.borrow()
            .as_ref()
            .expect(MSG)
            .task
            .get_host(addr)
            .unwrap()
    })
}

pub(crate) fn net_local_handle() -> crate::net::NetLocalHandle {
    let addr = TASK.with(|task| task.borrow().as_ref().expect(MSG).addr);
    CONTEXT.with(|ctx| ctx.borrow().as_ref().expect(MSG).net.get_host(addr))
}

pub(crate) fn fs_local_handle() -> crate::fs::FsLocalHandle {
    let addr = TASK.with(|task| task.borrow().as_ref().expect(MSG).addr);
    CONTEXT.with(|ctx| ctx.borrow().as_ref().expect(MSG).fs.local_handle(addr))
}

/// Set this [`Handle`] as the current active [`Handle`].
///
/// [`Handle`]: Handle
pub(crate) fn enter(new: Handle) -> EnterGuard {
    CONTEXT.with(|ctx| {
        let old = ctx.borrow_mut().replace(new);
        EnterGuard(old)
    })
}

pub(crate) struct EnterGuard(Option<Handle>);

impl Drop for EnterGuard {
    fn drop(&mut self) {
        CONTEXT.with(|ctx| {
            *ctx.borrow_mut() = self.0.take();
        });
    }
}

pub(crate) fn enter_task(new: Arc<TaskInfo>) -> TaskEnterGuard {
    TASK.with(|ctx| {
        let old = ctx.borrow_mut().replace(new);
        TaskEnterGuard(old)
    })
}

pub(crate) struct TaskEnterGuard(Option<Arc<TaskInfo>>);

impl Drop for TaskEnterGuard {
    fn drop(&mut self) {
        TASK.with(|ctx| {
            *ctx.borrow_mut() = self.0.take();
        });
    }
}

const MSG: &str =
    "there is no reactor running, must be called from the context of a Madsim runtime";
