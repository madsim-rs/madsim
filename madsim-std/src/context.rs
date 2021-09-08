//! Thread local runtime context
use crate::{Handle, LocalHandle};

use std::cell::RefCell;

thread_local! {
    static CONTEXT: RefCell<Option<Handle>> = RefCell::new(None);
    static LOCAL_CONTEXT: RefCell<Option<LocalHandle>> = RefCell::new(None);
}

pub(crate) fn current() -> Handle {
    CONTEXT.with(|ctx| ctx.borrow().clone().expect(MSG))
}

pub(crate) fn net_local_handle() -> crate::net::NetLocalHandle {
    LOCAL_CONTEXT.with(|ctx| ctx.borrow().as_ref().expect(MSG).net.clone())
}

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

pub(crate) fn enter_local(new: LocalHandle) -> EnterLocalGuard {
    LOCAL_CONTEXT.with(|ctx| {
        let old = ctx.borrow_mut().replace(new);
        EnterLocalGuard(old)
    })
}

pub(crate) struct EnterLocalGuard(Option<LocalHandle>);

impl Drop for EnterLocalGuard {
    fn drop(&mut self) {
        LOCAL_CONTEXT.with(|ctx| {
            *ctx.borrow_mut() = self.0.take();
        });
    }
}

const MSG: &str =
    "there is no reactor running, must be called from the context of a Madsim runtime";
