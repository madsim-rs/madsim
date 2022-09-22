/// Client-level context.
pub trait ClientContext: Send + Sync + 'static {}

/// An empty [`ClientContext`] that can be used when no customizations are needed.
#[derive(Clone, Debug, Default)]
pub struct DefaultClientContext;

impl ClientContext for DefaultClientContext {}
