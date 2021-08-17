use crate::kvraft::{
    msg::*,
    server::{Server, State},
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub type ShardKv = Server<Kv>;

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Kv {
    kv: HashMap<String, String>,
    // A circular queue with max capacity 50
    ids: Vec<u64>,
}

impl State for Kv {
    type Command = Op;
    type Output = String;

    fn apply(&mut self, id: u64, cmd: Self::Command) -> Self::Output {
        let unique = !self.ids.contains(&id);
        if self.ids.len() > 50 {
            self.ids.remove(0);
        }
        self.ids.push(id);
        match cmd {
            Op::Put { key, value } if unique => {
                self.kv.insert(key, value);
            }
            Op::Append { key, value } if unique => {
                self.kv.entry(key).or_default().push_str(&value);
            }
            Op::Get { key } => return self.kv.get(&key).cloned().unwrap_or_default(),
            _ => {}
        }
        "".into()
    }
}
