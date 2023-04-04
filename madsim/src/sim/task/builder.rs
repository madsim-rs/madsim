use std::future::Future;

use super::{JoinHandle, Spawner};

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

    /// Spawns `!Send` a task on the current `LocalSet` with this builder's settings.
    #[track_caller]
    pub fn spawn_local<Fut>(self, future: Fut) -> JoinHandle<Fut::Output>
    where
        Fut: Future + 'static,
        Fut::Output: 'static,
    {
        Spawner::current().spawn_inner(future, self.name)
    }
}
