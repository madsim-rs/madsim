use super::Log;
use serde::{Deserialize, Serialize};
use std::ops::{Index, RangeFrom};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Logs {
    offset: usize,
    logs: Vec<Log>,
}

impl Logs {
    pub fn new() -> Self {
        Logs {
            offset: 0,
            logs: vec![Log::default()],
        }
    }
    pub fn begin(&self) -> usize {
        self.offset
    }
    pub fn len(&self) -> usize {
        self.offset + self.logs.len()
    }
    pub fn get(&self, index: usize) -> Option<&Log> {
        index
            .checked_sub(self.offset)
            .and_then(|i| self.logs.get(i))
    }
    pub fn push(&mut self, log: Log) {
        self.logs.push(log);
    }
    pub fn extend_from_slice(&mut self, slice: &[Log]) {
        self.logs.extend_from_slice(slice);
    }
    pub fn truncate(&mut self, len: usize) {
        self.logs.truncate(len - self.offset);
    }
    pub fn last(&self) -> Option<&Log> {
        Some(self.logs.last().unwrap())
    }
    pub fn trim_start_until(&mut self, idx: usize) {
        if let Some(delta) = idx.checked_sub(self.offset) {
            self.offset = idx;
            self.logs.drain(0..delta.min(self.logs.len()));
        }
    }
    pub fn clear(&mut self) {
        self.logs.clear();
    }
}

impl Index<usize> for Logs {
    type Output = Log;
    fn index(&self, index: usize) -> &Self::Output {
        self.get(index).unwrap_or_else(|| {
            panic!(
                "index {} out of range {}..{}",
                index,
                self.offset,
                self.len()
            )
        })
    }
}

impl Index<RangeFrom<usize>> for Logs {
    type Output = [Log];
    fn index(&self, range: RangeFrom<usize>) -> &Self::Output {
        let start = range.start.checked_sub(self.offset).expect("out of range");
        &self.logs[start..]
    }
}
