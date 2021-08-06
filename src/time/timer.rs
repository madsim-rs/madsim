use super::Instant;
use std::{cmp::Ordering, collections::BinaryHeap};

#[derive(Default)]
pub struct Timer {
    queue: BinaryHeap<Event>,
}

impl Timer {
    pub fn push(&mut self, deadline: Instant, callback: impl FnOnce() + Send + Sync + 'static) {
        self.queue.push(Event {
            deadline,
            callback: Box::new(callback),
        });
    }

    pub fn next_time(&self) -> Option<Instant> {
        self.queue.peek().map(|e| e.deadline)
    }

    /// Fire all timers before or at the `time`.
    pub fn fire(&mut self, time: Instant) {
        while let Some(t) = self.next_time() {
            if t > time {
                break;
            }
            let event = self.queue.pop().unwrap();
            (event.callback)();
        }
    }
}

struct Event {
    deadline: Instant,
    callback: Box<dyn FnOnce() + Send + Sync + 'static>,
}

impl PartialEq for Event {
    fn eq(&self, other: &Self) -> bool {
        self.deadline.eq(&other.deadline)
    }
}

impl Eq for Event {}

impl PartialOrd for Event {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        other.deadline.partial_cmp(&self.deadline)
    }
}

impl Ord for Event {
    fn cmp(&self, other: &Event) -> Ordering {
        other.deadline.cmp(&self.deadline)
    }
}
