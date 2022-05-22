//! Collection types.

use crate::rand::{thread_rng, Rng};
use serde::{
    de::{Deserialize, Deserializer},
    ser::{Serialize, Serializer},
};
use std::borrow::Borrow;
use std::fmt::{self, Debug};
use std::hash::{BuildHasher, Hash};
use std::ops::{BitAnd, BitOr, BitXor, Deref, DerefMut, Index, Sub};
use std::panic::UnwindSafe;

pub use std::collections::{
    binary_heap, btree_map, btree_set, hash_map, hash_set, linked_list, vec_deque, BTreeMap,
    BTreeSet, BinaryHeap, LinkedList, TryReserveError, VecDeque,
};

/// A deterministic random state.
///
/// It can only be used within the madsim context.
#[derive(Clone, Debug)]
pub struct RandomState(ahash::RandomState);

/// Initialize a [`RandomState`] from global RNG.
impl Default for RandomState {
    fn default() -> Self {
        let mut rng = thread_rng();
        Self(ahash::RandomState::with_seeds(
            rng.gen(),
            rng.gen(),
            rng.gen(),
            rng.gen(),
        ))
    }
}

impl BuildHasher for RandomState {
    type Hasher = ahash::AHasher;
    #[inline]
    fn build_hasher(&self) -> Self::Hasher {
        self.0.build_hasher()
    }
}

/// A [`HashSet`](std::collections::HashSet) using [`RandomState`] to hash the items.
#[derive(Clone)]
pub struct HashSet<T, S = RandomState>(std::collections::HashSet<T, S>);

impl<T> HashSet<T, RandomState> {
    /// Creates an empty `HashSet`.
    pub fn new() -> Self {
        HashSet(std::collections::HashSet::with_hasher(
            RandomState::default(),
        ))
    }

    /// Creates an empty `HashSet` with the specified capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        HashSet(std::collections::HashSet::with_capacity_and_hasher(
            capacity,
            RandomState::default(),
        ))
    }
}

/// A [`HashMap`](std::collections::HashMap) using [`RandomState`](ahash::RandomState) to hash the items.
#[derive(Clone)]
pub struct HashMap<K, V, S = RandomState>(std::collections::HashMap<K, V, S>);

impl<K, V> HashMap<K, V, RandomState> {
    /// Creates an empty `HashMap`.
    pub fn new() -> Self {
        HashMap(std::collections::HashMap::with_hasher(
            RandomState::default(),
        ))
    }

    /// Creates an empty `HashMap` with the specified capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        HashMap(std::collections::HashMap::with_capacity_and_hasher(
            capacity,
            RandomState::default(),
        ))
    }
}

// The following code is borrowed from ahash.

impl<T, S> Deref for HashSet<T, S> {
    type Target = std::collections::HashSet<T, S>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T, S> DerefMut for HashSet<T, S> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<T, S> PartialEq for HashSet<T, S>
where
    T: Eq + Hash,
    S: BuildHasher,
{
    #[inline]
    fn eq(&self, other: &HashSet<T, S>) -> bool {
        self.0.eq(&other.0)
    }
}

impl<T, S> Eq for HashSet<T, S>
where
    T: Eq + Hash,
    S: BuildHasher,
{
}

impl<T, S> BitOr<&HashSet<T, S>> for &HashSet<T, S>
where
    T: Eq + Hash + Clone,
    S: BuildHasher + Default,
{
    type Output = HashSet<T, S>;
    #[inline]
    fn bitor(self, rhs: &HashSet<T, S>) -> HashSet<T, S> {
        HashSet(self.0.bitor(&rhs.0))
    }
}

impl<T, S> BitAnd<&HashSet<T, S>> for &HashSet<T, S>
where
    T: Eq + Hash + Clone,
    S: BuildHasher + Default,
{
    type Output = HashSet<T, S>;
    #[inline]
    fn bitand(self, rhs: &HashSet<T, S>) -> HashSet<T, S> {
        HashSet(self.0.bitand(&rhs.0))
    }
}

impl<T, S> BitXor<&HashSet<T, S>> for &HashSet<T, S>
where
    T: Eq + Hash + Clone,
    S: BuildHasher + Default,
{
    type Output = HashSet<T, S>;
    #[inline]
    fn bitxor(self, rhs: &HashSet<T, S>) -> HashSet<T, S> {
        HashSet(self.0.bitxor(&rhs.0))
    }
}

impl<T, S> Sub<&HashSet<T, S>> for &HashSet<T, S>
where
    T: Eq + Hash + Clone,
    S: BuildHasher + Default,
{
    type Output = HashSet<T, S>;
    #[inline]
    fn sub(self, rhs: &HashSet<T, S>) -> HashSet<T, S> {
        HashSet(self.0.sub(&rhs.0))
    }
}

impl<T, S> Debug for HashSet<T, S>
where
    T: Debug,
    S: BuildHasher,
{
    #[inline]
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(fmt)
    }
}

impl<T, const N: usize> From<[T; N]> for HashSet<T>
where
    T: Eq + Hash,
{
    #[inline]
    fn from(arr: [T; N]) -> Self {
        Self::from_iter(arr)
    }
}

impl<T, S> FromIterator<T> for HashSet<T, S>
where
    T: Eq + Hash,
    S: BuildHasher + Default,
{
    #[inline]
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> HashSet<T, S> {
        HashSet(std::collections::HashSet::from_iter(iter))
    }
}

impl<'a, T, S> IntoIterator for &'a HashSet<T, S> {
    type Item = &'a T;
    type IntoIter = hash_set::Iter<'a, T>;
    fn into_iter(self) -> Self::IntoIter {
        (&self.0).iter()
    }
}

impl<T, S> IntoIterator for HashSet<T, S> {
    type Item = T;
    type IntoIter = hash_set::IntoIter<T>;
    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<T, S> Extend<T> for HashSet<T, S>
where
    T: Eq + Hash,
    S: BuildHasher,
{
    #[inline]
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        self.0.extend(iter)
    }
}

impl<'a, T, S> Extend<&'a T> for HashSet<T, S>
where
    T: 'a + Eq + Hash + Copy,
    S: BuildHasher,
{
    #[inline]
    fn extend<I: IntoIterator<Item = &'a T>>(&mut self, iter: I) {
        self.0.extend(iter)
    }
}

impl<T> Default for HashSet<T, RandomState> {
    /// Creates an empty `HashSet<T, S>` with the `Default` value for the hasher.
    #[inline]
    fn default() -> HashSet<T, RandomState> {
        HashSet(std::collections::HashSet::default())
    }
}

impl<T> Serialize for HashSet<T>
where
    T: Serialize + Eq + Hash,
{
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.deref().serialize(serializer)
    }
}

impl<'de, T> Deserialize<'de> for HashSet<T>
where
    T: Deserialize<'de> + Eq + Hash,
{
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let hash_set = std::collections::HashSet::deserialize(deserializer);
        hash_set.map(|hash_set| Self(hash_set))
    }
}

impl<K, V, S> Deref for HashMap<K, V, S> {
    type Target = std::collections::HashMap<K, V, S>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<K, V, S> DerefMut for HashMap<K, V, S> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<K, V, S> UnwindSafe for HashMap<K, V, S>
where
    K: UnwindSafe,
    V: UnwindSafe,
{
}

impl<K, V, S> PartialEq for HashMap<K, V, S>
where
    K: Eq + Hash,
    V: PartialEq,
    S: BuildHasher,
{
    #[inline]
    fn eq(&self, other: &HashMap<K, V, S>) -> bool {
        self.0.eq(&other.0)
    }
}

impl<K, V, S> Eq for HashMap<K, V, S>
where
    K: Eq + Hash,
    V: Eq,
    S: BuildHasher,
{
}

impl<K, Q: ?Sized, V, S> Index<&Q> for HashMap<K, V, S>
where
    K: Eq + Hash + Borrow<Q>,
    Q: Eq + Hash,
    S: BuildHasher,
{
    type Output = V;

    /// Returns a reference to the value corresponding to the supplied key.
    ///
    /// # Panics
    ///
    /// Panics if the key is not present in the `HashMap`.
    #[inline]
    fn index(&self, key: &Q) -> &V {
        self.0.index(key)
    }
}

impl<K, V, S> Debug for HashMap<K, V, S>
where
    K: Debug,
    V: Debug,
    S: BuildHasher,
{
    #[inline]
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(fmt)
    }
}

impl<K, V, const N: usize> From<[(K, V); N]> for HashMap<K, V>
where
    K: Eq + Hash,
{
    #[inline]
    fn from(arr: [(K, V); N]) -> Self {
        Self::from_iter(arr)
    }
}

impl<K, V, S> FromIterator<(K, V)> for HashMap<K, V, S>
where
    K: Eq + Hash,
    S: BuildHasher + Default,
{
    #[inline]
    fn from_iter<T: IntoIterator<Item = (K, V)>>(iter: T) -> Self {
        HashMap(std::collections::HashMap::from_iter(iter))
    }
}

impl<'a, K, V, S> IntoIterator for &'a HashMap<K, V, S> {
    type Item = (&'a K, &'a V);
    type IntoIter = hash_map::Iter<'a, K, V>;
    fn into_iter(self) -> Self::IntoIter {
        (&self.0).iter()
    }
}

impl<'a, K, V, S> IntoIterator for &'a mut HashMap<K, V, S> {
    type Item = (&'a K, &'a mut V);
    type IntoIter = hash_map::IterMut<'a, K, V>;
    fn into_iter(self) -> Self::IntoIter {
        (&mut self.0).iter_mut()
    }
}

impl<K, V, S> IntoIterator for HashMap<K, V, S> {
    type Item = (K, V);
    type IntoIter = hash_map::IntoIter<K, V>;
    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<K, V, S> Extend<(K, V)> for HashMap<K, V, S>
where
    K: Eq + Hash,
    S: BuildHasher,
{
    #[inline]
    fn extend<T: IntoIterator<Item = (K, V)>>(&mut self, iter: T) {
        self.0.extend(iter)
    }
}

impl<'a, K, V, S> Extend<(&'a K, &'a V)> for HashMap<K, V, S>
where
    K: Eq + Hash + Copy + 'a,
    V: Copy + 'a,
    S: BuildHasher,
{
    #[inline]
    fn extend<T: IntoIterator<Item = (&'a K, &'a V)>>(&mut self, iter: T) {
        self.0.extend(iter)
    }
}

impl<K, V> Default for HashMap<K, V, RandomState> {
    #[inline]
    fn default() -> HashMap<K, V, RandomState> {
        HashMap::new()
    }
}

impl<K, V> Serialize for HashMap<K, V>
where
    K: Serialize + Eq + Hash,
    V: Serialize,
{
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.deref().serialize(serializer)
    }
}

impl<'de, K, V> Deserialize<'de> for HashMap<K, V>
where
    K: Deserialize<'de> + Eq + Hash,
    V: Deserialize<'de>,
{
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let hash_map = std::collections::HashMap::deserialize(deserializer);
        hash_map.map(|hash_map| Self(hash_map))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::Runtime;

    #[test]
    fn deterministic_hashset() {
        let mut seqs = BTreeSet::new();
        for i in 0..9 {
            let runtime = Runtime::with_seed_and_config(i / 3, crate::Config::default());
            let seq = runtime.block_on(async {
                let set = (0..10).collect::<HashSet<i32>>();
                set.into_iter().collect::<Vec<_>>()
            });
            seqs.insert(seq);
        }
        assert_eq!(seqs.len(), 3, "hashset is not deterministic");
    }

    #[test]
    fn deterministic_hashmap() {
        let mut seqs = BTreeSet::new();
        for i in 0..9 {
            let runtime = Runtime::with_seed_and_config(i / 3, crate::Config::default());
            let seq = runtime.block_on(async {
                let set = (0..10).map(|i| (i, i)).collect::<HashMap<_, _>>();
                set.into_iter().collect::<Vec<_>>()
            });
            seqs.insert(seq);
        }
        assert_eq!(seqs.len(), 3, "hashmap is not deterministic");
    }
}
