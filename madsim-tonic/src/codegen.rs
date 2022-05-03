//! Codegen exports used by `tonic-build`.

pub use async_trait::async_trait;
pub use futures_core;
pub use tower_service::Service;

pub use std::future::Future;
pub use std::pin::Pin;
pub use std::sync::Arc;
pub use std::task::{Context, Poll};

pub type StdError = Box<dyn std::error::Error + Send + Sync + 'static>;
pub type BoxFuture<T, E> = self::Pin<Box<dyn self::Future<Output = Result<T, E>> + Send + 'static>>;
pub type BoxStream<T> =
    self::Pin<Box<dyn futures_core::Stream<Item = Result<T, crate::Status>> + Send + 'static>>;
