use crate::{codegen::BoxMessage, Status};
use async_stream::try_stream;
use futures_util::{Stream, StreamExt};
use madsim::task::JoinHandle;
use std::{
    fmt,
    pin::Pin,
    task::{Context, Poll},
};
use tonic::codegen::BoxStream;

/// Streaming requests and responses.
pub struct Streaming<T> {
    stream: BoxStream<T>,
}

impl<T: Send + 'static> Streaming<T> {
    /// Creates a new streaming.
    ///
    /// The elements will be received from the endpoint starting with the given tag.
    /// If this is a bi-directional streaming RPC, `request_sending_task` is required.
    pub(crate) fn new(
        mut rx: madsim::net::Receiver,
        request_sending_task: Option<JoinHandle<()>>,
    ) -> Self {
        Streaming {
            stream: try_stream! {
                // For bi-directional streaming, we spawn a task to send requests.
                // This is used to cancel the task when the stream is dropped.
                let _task = request_sending_task.map(|t| t.cancel_on_drop());
                // receive messages
                while let Ok(msg) = rx.recv().await {
                    let msg = *msg.downcast::<Result<BoxMessage, Status>>().unwrap();
                    yield *msg?.downcast::<T>().unwrap();
                }
            }
            .boxed(),
        }
    }

    /// Creates a new streaming.
    ///
    /// This method is used by macros only. Not a public API.
    #[doc(hidden)]
    pub fn from_stream(stream: BoxStream<T>) -> Self {
        Streaming { stream }
    }
}

impl<T> Streaming<T> {
    /// Fetch the next message from this stream.
    pub async fn message(&mut self) -> Result<Option<T>, Status> {
        self.stream.next().await.transpose()
    }
}

impl<T> fmt::Debug for Streaming<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Streaming").finish()
    }
}

impl<T> Stream for Streaming<T> {
    type Item = Result<T, Status>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.stream.poll_next_unpin(cx)
    }
}
