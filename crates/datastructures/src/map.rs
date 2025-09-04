//! Implements map related data structures.

use ahash::RandomState;
use bytemuck::{Pod, Zeroable};
use rand::{rngs::ThreadRng, Rng};
use rostl_primitives::{
  cmov_body, cxchg_body, impl_cmov_for_generic_pod,
  traits::{Cmov, _Cmovbase},
};

use seq_macro::seq;

use crate::{array::MultiWayArray, queue::ShortQueue};

// Size of the insertion queue for deamortized insertions that failed.
const INSERTION_QUEUE_MAX_SIZE: usize = 10;
// Number of deamortized insertions to perform per insertion.
const DEAMORTIZED_INSERTIONS: usize = 2;
// Number of elements in each map bucket.
const BUCKET_SIZE: usize = 4;

use std::hash::Hash;

/// Utility trait for types that can be uses as keys in the map.
pub trait OHash: Cmov + Pod + Hash + PartialEq {}
// UNDONE(git-20): Hashmaps with boolean key aren't supported until boolean implements pod.s
impl<K> OHash for K where K: Cmov + Pod + Hash + PartialEq {}

/// An element in the map.
#[repr(align(8))]
#[repr(C)]
#[derive(Debug, Default, Clone, Copy, Zeroable)]
pub struct InlineElement<K, V>
where
  K: OHash,
  V: Cmov + Pod,
{
  key: K,
  value: V,
}
unsafe impl<K: OHash, V: Cmov + Pod> Pod for InlineElement<K, V> {}
impl_cmov_for_generic_pod!(InlineElement<K,V>; where K: OHash, V: Cmov + Pod);

/// A bucket in the map.
/// The bucket has `BUCKET_SIZE` elements.
/// # Invariants
/// * The elements in the bucket that have `is_valid == true` are non empty.
/// * The elements in the bucket that have `is_valid == false` are empty.
/// * No two valid elements have the same key.
#[derive(Debug, Default, Clone, Copy, Zeroable)]
#[repr(C)]
struct Bucket<K, V>
where
  K: OHash,
  V: Cmov + Pod,
{
  is_valid: [bool; BUCKET_SIZE],
  elements: [InlineElement<K, V>; BUCKET_SIZE],
}
unsafe impl<K: OHash, V: Cmov + Pod> Pod for Bucket<K, V> {}
impl_cmov_for_generic_pod!(Bucket<K,V>; where K: OHash, V: Cmov + Pod);

impl<K, V> Bucket<K, V>
where
  K: OHash,
  V: Cmov + Pod,
{
  const fn is_empty(&self, i: usize) -> bool {
    !self.is_valid[i]
  }

  /// Replaces the value of a Key, if:
  ///  1) `real`
  ///  2) the bucket has a valid element with the same key as `element`
  /// # Returns
  /// * `true` - if a replacement happened (including if it was with the same value)
  /// * `false` - otherwise (including if the element is empty)
  fn update_if_exists(&mut self, real: bool, element: InlineElement<K, V>) -> bool {
    let mut updated = false;
    for i in 0..BUCKET_SIZE {
      let choice = real & !self.is_empty(i) & (self.elements[i].key == element.key);
      self.elements[i].value.cmov(&element.value, choice);
      updated.cmov(&true, choice);
    }
    updated
  }

  fn read_if_exists(&self, key: K, ret: &mut V) -> bool {
    let mut found = false;
    for i in 0..BUCKET_SIZE {
      let choice = !self.is_empty(i) & (self.elements[i].key == key);
      ret.cmov(&self.elements[i].value, choice);
      found.cmov(&true, choice);
    }
    found
  }

  /// Insert an element into the bucket uf it has an empty slot, otherwise does nothing.
  /// # Preconditions
  /// * real ==> The same key isn't in the bucket.
  /// # Returns
  /// * `true` - if the element was inserted or if `real == false`
  /// * `false` - otherwise
  fn insert_if_available(&mut self, real: bool, element: InlineElement<K, V>) -> bool {
    let mut inserted = !real;
    for i in 0..BUCKET_SIZE {
      let choice = !inserted & self.is_empty(i);
      self.is_valid[i].cmov(&true, choice);
      self.elements[i].cmov(&element, choice);
      inserted.cmov(&true, choice);
    }
    inserted
  }
}

/// An unsorted map that is oblivious to the access pattern.
/// The map uses cuckoo hashing with size-2 buckets, two tables and a deamortization queue.
/// `INSERTION_QUEUE_MAX_SIZE` is the maximum size of the deamortization queue.
/// `DEAMORTIZED_INSERTIONS` is the number of deamortized insertions to perform per insert call.
/// # Invariants
/// * A key appears at most once in a valid element in between the two tables and the insertion queue.
/// * The two tables have the same capacity.
/// * The two tables have a different hash functions (different keys on the keyed hash function).
#[derive(Debug)]
pub struct UnsortedMap<K, V>
where
  K: OHash + Default + std::fmt::Debug,
  V: Cmov + Pod + Default + std::fmt::Debug,
{
  /// Number of elements in the map
  size: usize,
  /// Maximum number of elements for perfect load `(max_size / load_factor)`
  _capacity: usize,
  /// Maximum number of entries in each table `(buckets / load_factor)`
  table_size: usize,
  /// The two tables
  table: MultiWayArray<Bucket<K, V>, 2>,
  /// The hasher used to hash keys
  hash_builders: [RandomState; 2],
  /// The insertion queue
  insertion_queue: ShortQueue<InlineElement<K, V>, INSERTION_QUEUE_MAX_SIZE>,
  /// Random source for random indices
  rng: ThreadRng,
}

impl<K, V> UnsortedMap<K, V>
where
  K: OHash + Default + std::fmt::Debug,
  V: Cmov + Pod + Default + std::fmt::Debug,
{
  /// Creates a new `UnsortedMap` with the given capacity `n`.
  pub fn new(capacity: usize) -> Self {
    debug_assert!(capacity > 0);
    // For load factor of 0.8: cap / (0.8 * BUCKET_SIZE) = cap * 5 / (4 * BUCKET_SIZE)
    let table_size = (capacity * 5).div_ceil(4 * BUCKET_SIZE).max(2);
    Self {
      size: 0,
      _capacity: capacity,
      table_size,
      table: MultiWayArray::new(table_size),
      hash_builders: [RandomState::new(), RandomState::new()],
      insertion_queue: ShortQueue::new(),
      rng: rand::rng(),
    }
  }

  #[inline(always)]
  fn hash_key<const TABLE: usize>(&self, key: &K) -> usize {
    (self.hash_builders[TABLE].hash_one(key) % self.table_size as u64) as usize
  }

  /// Tries to get a value from the map.
  /// # Returns
  /// * `true` if the key was found
  /// * `false` if the key wasn't found
  /// # Postconditions
  /// * If the key was found, the value is written to `ret`
  /// * If the key wasn't found, `ret` is not modified
  pub fn get(&mut self, key: K, ret: &mut V) -> bool {
    let mut found = false;
    let mut tmp: Bucket<K, V> = Default::default();

    // Tries to get the element from each table:
    // seq! does manual loop unrolling in rust. We need it to be able to use the constant INDEX in the hash_key function.
    seq!(INDEX in 0..2 {
      let hash = self.hash_key::<INDEX>(&key);
      self.table.read(INDEX, hash, &mut tmp);
      let found_local = tmp.read_if_exists(key, ret);
      found.cmov(&true, found_local);
    });

    // Tries to get the element from the deamortization queue:
    for i in 0..self.insertion_queue.size {
      let element = self.insertion_queue.elements.data[i];
      let found_local = !element.is_empty() & (element.value.key == key);
      ret.cmov(&element.value.value, found_local);
      found.cmov(&true, found_local);
    }

    found
  }

  /// Tries to insert an element into some of the hash tables, in case of collisions, the element is replaced.
  /// # Returns
  /// * `true` if it was possible to insert into an empty slot or `real == false`.
  /// * `false` if the element was replaced and therefore the new element value needs to be inserted into the insertion queue.
  fn try_insert_entry(&mut self, real: bool, element: &mut InlineElement<K, V>) -> bool {
    let mut done = !real;

    seq!(INDEX_REV in 0..2 {{
      #[allow(clippy::identity_op, clippy::eq_op)] // False positives due to the seq! macro.
      const INDEX: usize = 1 - INDEX_REV;

      let hash = self.hash_key::<INDEX>(&element.key);
      self.table.update(INDEX, hash, |bucket| {
        let choice = !done;
        let inserted = bucket.insert_if_available(choice, *element);
        done.cmov(&true, inserted);
        let randidx = self.rng.random_range(0..BUCKET_SIZE);
        bucket.elements[randidx].cxchg(element, !done);
      });
    }});
    done
  }

  /// Deamortizes the insertion queue by trying to insert elements into the tables.
  pub fn deamortize_insertion_queue(&mut self) {
    for _ in 0..DEAMORTIZED_INSERTIONS {
      let mut element = InlineElement::default();
      let real = self.insertion_queue.size > 0;

      // Use FIFO order so we don't get stuck in a loop in the random graph of cuckoo hashing.
      self.insertion_queue.maybe_pop(real, &mut element);
      let has_pending_element = !self.try_insert_entry(real, &mut element);
      self.insertion_queue.maybe_push(has_pending_element, element);
    }
  }

  /// Inserts an elementn into the map. If the insertion doesn't finish, the removed element is inserted into the insertion queue.
  /// # Preconditions
  /// * The key is not in the map already.
  pub fn insert(&mut self, key: K, value: V) {
    // UNDONE(git-32): Recover in case the insertion queue is full.
    assert!(self.insertion_queue.size < INSERTION_QUEUE_MAX_SIZE);
    self.insertion_queue.maybe_push(true, InlineElement { key, value });
    self.deamortize_insertion_queue();
    self.size.cmov(&(self.size + 1), true);
  }

  /// Conditionally inserts an element into the map obliviously. If the insertion doesn't finish, the removed element is inserted into the insertion queue.
  /// # Preconditions
  /// * If `real` is true, the key is not in the map already.
  /// # Parameters
  /// * `real` - if true, the insertion is performed, if false, the
  ///   insertion is a dummy insertion that doesn't modify the logical map.
  pub fn insert_cond(&mut self, key: K, value: V, real: bool) {
    // UNDONE(git-32): Recover in case the insertion queue is full.
    assert!(self.insertion_queue.size < INSERTION_QUEUE_MAX_SIZE);
    self.insertion_queue.maybe_push(real, InlineElement { key, value });
    self.deamortize_insertion_queue();
    self.size.cmov(&(self.size + 1), real);
  }

  /// Updates a value that is already in the map.
  /// # Preconditions
  /// * The key is in the map.
  /// # Returns
  /// * `true` if the value was updated
  pub fn write(&mut self, key: K, value: V) {
    let mut updated = false;

    // Tries to get the element from each table:
    // seq! does manual loop unrolling in rust. We need it to be able to use the constant INDEX in the hash_key function.
    seq!(INDEX in 0..2 {
      let hash = self.hash_key::<INDEX>(&key);
      self.table.update(INDEX, hash, |bucket| {
        let choice = !updated;
        let updated_local = bucket.update_if_exists(choice, InlineElement { key, value });
        updated.cmov(&true, updated_local);
      });
    });

    // Tries to get the element from the deamortization queue:
    for i in 0..self.insertion_queue.size {
      let element = &mut self.insertion_queue.elements.data[i];
      let choice = !updated & !element.is_empty() & (element.value.key == key);
      element.value.value.cmov(&value, choice);
      updated.cmov(&true, choice);
    }

    assert!(updated);
  }

  // UNDONE(git-33): add efficient upsert function (inserts if not present, updates if present)
  // UNDONE(git-33): add delete function
}

// UNDONE(git-35): Add benchmarks for the map.

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_unsorted_map() {
    let mut map: UnsortedMap<u32, u32> = UnsortedMap::new(2);
    assert_eq!(map.size, 0);
    let mut value = 0;
    assert!(!map.get(1, &mut value));
    map.insert(1, 2);
    assert_eq!(map.size, 1);
    assert!(map.get(1, &mut value));
    assert_eq!(value, 2);
    map.write(1, 3);
    assert!(map.get(1, &mut value));
    assert_eq!(value, 3);
  }

  #[test]
  fn test_full_map() {
    const SZ: usize = 1024;
    let mut map: UnsortedMap<u32, u32> = UnsortedMap::new(SZ);
    assert_eq!(map.size, 0);
    for i in 0..SZ as u32 {
      map.insert(i, i * 2);
      let mut value = 0;
      assert!(map.get(i, &mut value));
      assert_eq!(value, i * 2);
      assert_eq!(map.size, (i + 1) as usize);
      map.write(i, i * 3);
      assert!(map.get(i, &mut value));
      assert_eq!(value, i * 3);
      assert_eq!(map.size, (i + 1) as usize);
    }
  }

  #[test]
  fn test_insert_cond() {
    // Test that conditional insert works when real is true and doesn't when real is false
    let mut map: UnsortedMap<u32, u32> = UnsortedMap::new(8);
    assert_eq!(map.size, 0);

    // Dummy insertion (real = false) should not increase logical size nor make the key visible
    map.insert_cond(10, 100, false);
    assert_eq!(map.size, 0);
    let mut value = 0;
    assert!(!map.get(10, &mut value));

    // Real insertion should store the value and increase size
    map.insert_cond(10, 200, true);
    assert_eq!(map.size, 1);
    assert!(map.get(10, &mut value));
    assert_eq!(value, 200);

    // Another dummy insert with different value should not change stored value
    map.insert_cond(10, 300, false);
    assert_eq!(map.size, 1);
    assert!(map.get(10, &mut value));
    assert_eq!(value, 200);
  }

  fn test_map_subtypes<
    K: OHash + Default + std::fmt::Debug,
    V: Cmov + Pod + Default + std::fmt::Debug,
  >() {
    const SZ: usize = 1024;
    let mut map: UnsortedMap<K, V> = UnsortedMap::new(SZ);
    assert_eq!(map.size, 0);
    let mut value = V::default();
    assert!(!map.get(K::default(), &mut value));
    map.insert(K::default(), V::default());
    assert_eq!(map.size, 1);
    assert!(map.get(K::default(), &mut value));
  }

  #[test]
  fn test_map_multiple_types() {
    test_map_subtypes::<u32, u32>();
    test_map_subtypes::<u64, u64>();
    test_map_subtypes::<u128, u128>();
    test_map_subtypes::<i32, i32>();
    test_map_subtypes::<i64, i64>();
    test_map_subtypes::<i128, i128>();
  }

  // UNDONE(git-34): Add further tests for the map.
}
