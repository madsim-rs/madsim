use std::{future::Future, time::Duration};

#[derive(Debug, Clone)]
pub(crate) struct TimeHandle {}

impl TimeHandle {
    pub fn new() -> Self {
        TimeHandle {}
    }

    pub fn sleep(&self, duration: Duration) -> impl Future<Output = ()> {
        tokio::time::sleep(duration)
    }
}

pub fn sleep(duration: Duration) -> impl Future<Output = ()> {
    tokio::time::sleep(duration)
}
