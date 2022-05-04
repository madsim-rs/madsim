use crate::Status;

/// Streaming requests and responses.
pub struct Streaming<T> {
    message: T,
}

impl<T> Streaming<T> {
    /// Fetch the next message from this stream.
    pub async fn message(&mut self) -> Result<Option<T>, Status> {
        todo!()
    }
}
