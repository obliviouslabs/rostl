//! Implements map related data structures.

use ahash::RandomState;
use bytemuck::{Pod, Zeroable};
use rand::{rngs::ThreadRng, Rng};
use rods_primitives::{
  cmov_body, impl_cmov_for_generic_pod,
  traits::{Cmov, _Cmovbase, cswap},
};
use seq_macro::seq;

use crate::{array::DynamicArray, queue::ShortQueue};

// Size of the insertion queue for deamortized insertions that failed.
const INSERTION_QUEUE_MAX_SIZE: usize = 20;
// Number of deamortized insertions to perform per insertion.
const DEAMORTIZED_INSERTIONS: usize = 2;
// Number of elements in each map bucket.
const BUCKET_SIZE: usize = 2;

use std::hash::Hash;

/// Utility trait for types that can be uses as keys in the map.
pub trait OHash: Cmov + Pod + Hash + PartialEq {}
// UNDONE(git-20): Hashmaps with boolean key aren't supported until boolean implements pod.s
impl<K> OHash for K where K: Cmov + Pod + Hash + PartialEq {}

/// An element in the map.
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

#[derive(Debug, Default, Clone, Copy, Zeroable)]
/// A struct that represents an element in a bucket.
pub struct BucketElement<K, V>
where
  K: OHash,
  V: Cmov + Pod,
{
  is_valid: bool,
  element: InlineElement<K, V>,
}
unsafe impl<K: OHash, V: Cmov + Pod> Pod for BucketElement<K, V> {}
impl_cmov_for_generic_pod!(BucketElement<K,V>; where K: OHash, V: Cmov + Pod);

impl<K, V> BucketElement<K, V>
where
  K: OHash,
  V: Cmov + Pod,
{
  #[inline(always)]
  const fn is_empty(&self) -> bool {
    !self.is_valid
  }
}

/// A bucket in the map.
/// The bucket has `BUCKET_SIZE` elements.
/// # Invariants
/// * The elements in the bucket that have `is_valid == true` are non empty.
/// * The elements in the bucket that have `is_valid == false` are empty.
/// * No two valid elements have the same key.
#[derive(Debug, Default, Clone, Copy, Zeroable)]
struct Bucket<K, V>
where
  K: OHash,
  V: Cmov + Pod,
{
  elements: [BucketElement<K, V>; BUCKET_SIZE],
}
unsafe impl<K: OHash, V: Cmov + Pod> Pod for Bucket<K, V> {}
impl_cmov_for_generic_pod!(Bucket<K,V>; where K: OHash, V: Cmov + Pod);

impl<K, V> Bucket<K, V>
where
  K: OHash,
  V: Cmov + Pod,
{
  /// Replaces the value of a Key, if:
  ///  1) `real`
  ///  2) the bucket has a valid element with the same key as `element`
  /// # Returns
  /// * `true` - if a replacement happened (including if it was with the same value)
  /// * `false` - otherwise (including if the element is empty)
  fn update_if_exists(&mut self, real: bool, element: InlineElement<K, V>) -> bool {
    let mut updated = false;
    for i in 0..BUCKET_SIZE {
      let element_i = &mut self.elements[i];
      let choice = real & !element_i.is_empty() & (element_i.element.key == element.key);
      element_i.element.value.cmov(&element.value, choice);
      updated.cmov(&true, choice);
    }
    updated
  }

  fn read_if_exists(&self, key: K, ret: &mut V) -> bool {
    let mut found = false;
    for i in 0..BUCKET_SIZE {
      let element_i = &self.elements[i];
      let choice = !element_i.is_empty() & (element_i.element.key == key);
      ret.cmov(&element_i.element.value, choice);
      found.cmov(&true, choice);
    }
    found
  }

  /// Insert an element into the bucket uf it has an empty slot, otherwise does nothing.
  /// # Preconditions
  /// * real ==> The same key isn't in the bucket.
  /// # Returns
  /// * `true` - if the element was inserted or if real = false
  /// * `false` - otherwise
  fn insert_if_available(&mut self, real: bool, element: InlineElement<K, V>) -> bool {
    let mut inserted = !real;
    for i in 0..BUCKET_SIZE {
      let element_i = &mut self.elements[i];
      let choice = !inserted & element_i.is_empty();
      element_i.element.cmov(&element, choice);
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
  /// capacity
  capacity: usize,
  /// The two tables
  table: [DynamicArray<Bucket<K, V>>; 2],
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
    Self {
      size: 0,
      capacity,
      table: [DynamicArray::new(capacity), DynamicArray::new(capacity)],
      hash_builders: [RandomState::new(), RandomState::new()],
      insertion_queue: ShortQueue::new(),
      rng: rand::rng(),
    }
  }

  #[inline(always)]
  fn hash_key<const TABLE: usize>(&self, key: &K) -> usize {
    (self.hash_builders[TABLE].hash_one(key) % self.capacity as u64) as usize
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
      let table = &mut self.table[INDEX];
      table.read(hash, &mut tmp);
      let found_local = tmp.read_if_exists(key, ret);
      found.cmov(&found_local, true);
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
  /// * `true` if it was possible to insert into an empty slot
  /// * `false` if the element was replaced and therefore the new element value needs to be inserted into the insertion queue.
  fn try_insert_entry(&mut self, real: bool, element: &mut InlineElement<K, V>) -> bool {
    let mut done = !real;

    seq!(INDEX_REV in 0..2 {{
      #[allow(clippy::identity_op, clippy::eq_op)] // False positives due to the seq! macro.
      const INDEX: usize = 1 - INDEX_REV;

      let hash = self.hash_key::<INDEX>(&element.key);
      let table = &mut self.table[INDEX];
      table.update(hash, |bucket| {
        let choice = !done;
        let inserted = bucket.insert_if_available(choice, *element);
        done.cmov(&true, inserted);
        let randidx = self.rng.random_range(0..BUCKET_SIZE);
        cswap(&mut bucket.elements[randidx].element, element, !done);
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
      let has_trail = self.try_insert_entry(real, &mut element);
      self.insertion_queue.maybe_push(has_trail, element);
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
      let table = &mut self.table[INDEX];
      table.update(hash, |bucket| {
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
    let mut map: UnsortedMap<u32, u32> = UnsortedMap::new(10);
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

  // UNDONE(git-34): Add further tests for the map.
}
