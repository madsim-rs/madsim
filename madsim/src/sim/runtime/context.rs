//! Thread local runtime context
use crate::{
    runtime::Handle,
    task::{NodeId, TaskInfo},
};

use std::{cell::RefCell, sync::Arc};

thread_local! {
    static CONTEXT: RefCell<Option<Handle>> = const { RefCell::new(None) };
    static TASK: RefCell<Option<Arc<TaskInfo>>> = const { RefCell::new(None) };
}

pub(crate) fn current<T>(map: impl FnOnce(&Handle) -> T) -> T {
    CONTEXT.with(move |ctx| map(ctx.borrow().as_ref().expect(MSG)))
}

pub(crate) fn try_current<T>(map: impl FnOnce(&Handle) -> T) -> Option<T> {
    // note: TLS may be deallocated
    CONTEXT
        .try_with(move |ctx| ctx.borrow().as_ref().map(map))
        .ok()
        .flatten()
}

pub(crate) fn current_task() -> Arc<TaskInfo> {
    TASK.with(|task| task.borrow().clone().expect(MSG))
}

pub(crate) fn try_current_task() -> Option<Arc<TaskInfo>> {
    TASK.try_with(|task| task.borrow().clone()).ok().flatten()
}

pub(crate) fn current_node() -> NodeId {
    TASK.with(|task| task.borrow().as_ref().expect(MSG).node.id)
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
        let _span = new.span.clone().entered();
        let old = ctx.borrow_mut().replace(new);
        TaskEnterGuard { old, _span }
    })
}

pub(crate) struct TaskEnterGuard {
    old: Option<Arc<TaskInfo>>,
    _span: tracing::span::EnteredSpan,
}

impl Drop for TaskEnterGuard {
    fn drop(&mut self) {
        TASK.with(|ctx| {
            *ctx.borrow_mut() = self.old.take();
        });
    }
}

const MSG: &str =
    "there is no reactor running, must be called from the context of a Madsim runtime";
