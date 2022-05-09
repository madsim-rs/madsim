use crate::Status;
use madsim::task::Task;
use std::{fmt, marker::PhantomData, sync::Arc};

/// Streaming requests and responses.
pub struct Streaming<T> {
    ep: Arc<madsim::net::Endpoint>,
    /// Tag of the next message to receive.
    tag: u64,
    _mark: PhantomData<T>,
    /// For bi-directional streaming, we spawn a task to send requests.
    /// This is used to cancel the task when the stream is dropped.
    _request_sending_task: Option<Task<()>>,
}

impl<T> Streaming<T> {
    /// Creates a new streaming.
    ///
    /// The elements will be received from the endpoint starting with the given tag.
    /// If this is a bi-directional streaming RPC, `request_sending_task` is required.
    pub(crate) fn new(
        ep: Arc<madsim::net::Endpoint>,
        tag: u64,
        request_sending_task: Option<Task<()>>,
    ) -> Self {
        Streaming {
            ep,
            tag,
            _mark: PhantomData,
            _request_sending_task: request_sending_task,
        }
    }
}

/// A marker type that indicates the stream is end.
pub(crate) struct StreamEnd;

impl<T: 'static> Streaming<T> {
    /// Fetch the next message from this stream.
    pub async fn message(&mut self) -> Result<Option<T>, Status> {
        let (rsp, _) = self.ep.recv_from_raw(self.tag).await?;
        if rsp.downcast_ref::<StreamEnd>().is_some() {
            return Ok(None);
        }
        let rsp = *rsp.downcast::<T>().expect("message type mismatch");
        self.tag += 1;
        Ok(Some(rsp))
    }
}

impl<T> fmt::Debug for Streaming<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Streaming").finish()
    }
}
