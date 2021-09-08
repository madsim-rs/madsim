//! Thread local runtime context
use crate::Handle;

use std::{cell::RefCell, net::SocketAddr};

thread_local! {
    static CONTEXT: RefCell<Option<Handle>> = RefCell::new(None);
    static ADDR: RefCell<Option<SocketAddr>> = RefCell::new(None);
}

pub(crate) fn current() -> Handle {
    CONTEXT.with(|ctx| ctx.borrow().clone().expect(MSG))
}

#[allow(dead_code)]
pub(crate) fn current_addr() -> Option<SocketAddr> {
    ADDR.with(|addr| *addr.borrow())
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
    let addr = ADDR.with(|addr| addr.borrow().expect(MSG));
    CONTEXT.with(|ctx| ctx.borrow().as_ref().expect(MSG).task.local_handle(addr))
}

pub(crate) fn net_local_handle() -> crate::net::NetLocalHandle {
    let addr = ADDR.with(|addr| addr.borrow().expect(MSG));
    CONTEXT.with(|ctx| ctx.borrow().as_ref().expect(MSG).net.get_host(addr))
}

pub(crate) fn fs_local_handle() -> crate::fs::FsLocalHandle {
    let addr = ADDR.with(|addr| addr.borrow().expect(MSG));
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

pub(crate) fn enter_task(new: SocketAddr) -> TaskEnterGuard {
    ADDR.with(|ctx| {
        let old = ctx.borrow_mut().replace(new);
        TaskEnterGuard(old)
    })
}

pub(crate) struct TaskEnterGuard(Option<SocketAddr>);

impl Drop for TaskEnterGuard {
    fn drop(&mut self) {
        ADDR.with(|ctx| {
            *ctx.borrow_mut() = self.0.take();
        });
    }
}

const MSG: &str =
    "there is no reactor running, must be called from the context of a Madsim runtime";
