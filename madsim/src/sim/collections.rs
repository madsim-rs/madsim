//! Collection types.

pub use ahash::AHashMap as HashMap;
pub use ahash::AHashSet as HashSet;
pub use std::collections::{
    binary_heap, btree_map, btree_set, linked_list, vec_deque, BTreeMap, BTreeSet, BinaryHeap,
    LinkedList, TryReserveError, VecDeque,
};
// FIXME: should use `hash_map` and `hash_set` of ahash
pub use std::collections::{hash_map, hash_set};
