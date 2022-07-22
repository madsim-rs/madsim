//! Thread local runtime context
use crate::{
    runtime::Handle,
    task::{NodeId, TaskInfo},
};

use std::{cell::RefCell, sync::Arc};

thread_local! {
    static CONTEXT: RefCell<Option<Handle>> = RefCell::new(None);
    static TASK: RefCell<Option<Arc<TaskInfo>>> = RefCell::new(None);
}

pub(crate) fn current<T>(map: impl FnOnce(&Handle) -> T) -> T {
    CONTEXT.with(move |ctx| map(ctx.borrow().as_ref().expect(MSG)))
}

pub(crate) fn try_current<T>(map: impl FnOnce(&Handle) -> T) -> Option<T> {
    CONTEXT.with(move |ctx| ctx.borrow().as_ref().map(map))
}

pub(crate) fn current_task() -> Arc<TaskInfo> {
    TASK.with(|task| task.borrow().clone().expect(MSG))
}

pub(crate) fn try_current_task() -> Option<Arc<TaskInfo>> {
    TASK.with(|task| task.borrow().clone())
}

pub(crate) fn current_node() -> NodeId {
    TASK.with(|task| task.borrow().as_ref().expect(MSG).node)
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
