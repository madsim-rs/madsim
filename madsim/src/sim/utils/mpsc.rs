//! A multi-producer, single-consumer queue but allows
//! consumer to randomly choose an element from the queue.

use crate::rand::GlobalRng;
use rand::Rng;
use std::{
    fmt,
    sync::{Arc, Mutex, Weak},
};

/// Creates a new asynchronous channel, returning the sender/receiver halves.
pub fn channel<T>() -> (Sender<T>, Receiver<T>) {
    let inner = Arc::new(Inner {
        queue: Mutex::new(Vec::new()),
    });
    let sender = Sender {
        inner: Arc::downgrade(&inner),
    };
    let recver = Receiver { inner };
    (sender, recver)
}

/// The sending-half of Rust’s asynchronous [`channel`] type.
pub struct Sender<T> {
    inner: Weak<Inner<T>>,
}

/// The receiving half of Rust’s [`channel`] type.
pub struct Receiver<T> {
    inner: Arc<Inner<T>>,
}

struct Inner<T> {
    queue: Mutex<Vec<T>>,
}

impl<T> Clone for Sender<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<T> fmt::Debug for SendError<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SendError").finish_non_exhaustive()
    }
}

/// An error returned from the `Sender::send` function on channels.
pub struct SendError<T>(pub T);

impl<T> Sender<T> {
    /// Attempts to send a value on this channel, returning it back if it could not be sent.
    pub fn send(&self, value: T) -> Result<(), SendError<T>> {
        if let Some(inner) = self.inner.upgrade() {
            inner.queue.lock().unwrap().push(value);
            Ok(())
        } else {
            Err(SendError(value))
        }
    }
}

/// This enumeration is the list of the possible reasons
/// that `try_recv_random` could not return data when called.
pub enum TryRecvError {
    Empty,
    Disconnected,
}

impl<T> Receiver<T> {
    /// Attempts to return a pending value on this receiver without blocking.
    pub fn try_recv_random(&self, rng: &GlobalRng) -> Result<T, TryRecvError> {
        let mut queue = self.inner.queue.lock().unwrap();
        if !queue.is_empty() {
            let idx = rng.with(|rng| rng.gen_range(0..queue.len()));
            Ok(queue.swap_remove(idx))
        } else if Arc::weak_count(&self.inner) == 0 {
            Err(TryRecvError::Disconnected)
        } else {
            Err(TryRecvError::Empty)
        }
    }
}
