use std::{collections::BTreeMap, fmt};

use super::*;

/// Runtime metrics.
pub struct RuntimeMetrics {
    pub(super) task: task::TaskHandle,
}

impl fmt::Debug for RuntimeMetrics {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RuntimeMetrics")
            .field("num_nodes", &self.num_nodes())
            .field("num_tasks", &self.num_tasks())
            .field("num_tasks_by_node", &self.num_tasks_by_node())
            .finish()
    }
}

impl RuntimeMetrics {
    /// Returns the number of nodes.
    pub fn num_nodes(&self) -> usize {
        self.task.num_nodes()
    }

    /// Returns the number of tasks.
    pub fn num_tasks(&self) -> usize {
        self.task.num_tasks()
    }

    /// Returns the number of tasks by node.
    pub fn num_tasks_by_node(&self) -> BTreeMap<String, usize> {
        self.task.num_tasks_by_node()
    }

    /// Returns the statistics of tasks by node by spawn.
    pub fn num_tasks_by_node_by_spawn(&self) -> String {
        self.task.num_tasks_by_node_by_spawn()
    }
}
