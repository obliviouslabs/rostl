//! Implements map related data structures.

use std::sync::mpsc;
use ahash::RandomState;
use bytemuck::{Pod, Zeroable};
use rayon::prelude::*;
use rods_primitives::{
  cmov_body, cxchg_body, impl_cmov_for_generic_pod,
  ooption::OOption,
  traits::{Cmov, _Cmovbase},
};
use rods_sort::{bitonic::bitonic_sort, compaction::compact};

use crate::map::{OHash, UnsortedMap};

// Size of the insertion queue for deamortized insertions that failed.
const INSERTION_QUEUE_MAX_SIZE: usize = 20;
// Number of deamortized insertions to perform per insertion.
const DEAMORTIZED_INSERTIONS: usize = 2;
// Number of elements in each map bucket.
const BUCKET_SIZE: usize = 2;
// Number of partitions in the map.
const P: usize = 15;

use std::mem::MaybeUninit;


/// The command sent to the worker thread to perform a batch operation.
///
enum Cmd<K, V, const B: usize>
where
  K: OHash + Pod + Default + std::fmt::Debug + Ord,
  V: Cmov + Pod + Default + std::fmt::Debug,
  BatchBlock<K, V>: Ord + Send,
{
  /// Get a batch of blocks from the map.
  Get {
    blocks: Box<[BatchBlock<K, V>; B]>,
    ret_tx: mpsc::Sender<Box<[BatchBlock<K, V>; B]>>,
  },
  /// Insert a batch of blocks into the map.
  Insert {
    blocks: Box<[BatchBlock<K, V>; B]>,
    ret_tx: mpsc::Sender<()>,
  },
  /// Shutdown the worker thread.
  Shutdown,
}

/// A sharded hashmap implementation.
/// The map is split across multiple partitions and each partition is a separate hashmap.
/// Queries are resolved in batches, to not leak the number of queries that go to each partition.
/// # Parameters
/// * `K`: The type of the keys in the map.
/// * `V`: The type of the values in the map.
/// * `P`: The number of partitions in the map.
/// * `B`: The maximum number of non-distinct keys in any partition in a batch.
#[derive(Debug)]
pub struct ShardedMap<K, V>
where
  K: OHash + Pod + Default + std::fmt::Debug + Send + Ord,
  V: Cmov + Pod + Default + std::fmt::Debug + Send,
{
  /// Number of elements in the map
  size: usize,
  /// capacity
  capacity: usize,
  /// The number of partitions in the map.
  num_partitions: usize,
  /// The partitions of the map.
  partitions: [UnsortedMap<K, V>; P],
  /// The random state used for hashing.
  random_state: RandomState,
}

#[repr(C)]
#[derive(Default, Debug, Clone, Copy, Zeroable, PartialEq, Eq, PartialOrd, Ord)]
struct BatchBlock<K, V>
where
  K: OHash + Pod + Default + std::fmt::Debug + Ord,
  V: Cmov + Pod + Default + std::fmt::Debug,
{
  index: usize,
  k: K,
  v: OOption<V>,
}
unsafe impl<K, V> Pod for BatchBlock<K, V>
where
  K: OHash + Pod + Default + std::fmt::Debug + Ord,
  V: Cmov + Pod + Default + std::fmt::Debug,
{
}
impl_cmov_for_generic_pod!(BatchBlock<K, V>; where K: OHash + Pod + Default + std::fmt::Debug + Ord, V: Cmov + Pod + Default + std::fmt::Debug);

impl<K, V> ShardedMap<K, V>
where
  K: OHash + Default + std::fmt::Debug + Send + Ord,
  V: Cmov + Pod + Default + std::fmt::Debug + Send,
  BatchBlock<K, V>: Ord + Send,
  UnsortedMap<K, V>: Send + Sync,
{
  /// Creates a new `ShardedMap` with the given number of partitions.
  pub fn new(capacity: usize) -> Self {
    let capacity_per_partition = (capacity + P - 1) / P;
    let capacity_rounded = capacity_per_partition * P;
    let mut partitions: [MaybeUninit<UnsortedMap<K, V>>; P] =
      unsafe { MaybeUninit::uninit().assume_init() };
    // let mut partitions :[UnsortedMap<K, V>; P] = unsafe { std::mem::zeroed() };
    for slot in partitions.iter_mut() {
      slot.write(UnsortedMap::new(capacity_per_partition));
    }
    let partitions =
      unsafe { std::ptr::read(&partitions as *const _ as *const [UnsortedMap<K, V>; P]) };
    // for i in 0..P {
    // partitions[i] = UnsortedMap::new(capacity_per_partition);
    // }
    Self {
      size: 0,
      capacity: capacity_rounded,
      num_partitions: P,
      partitions,
      random_state: RandomState::new(),
    }
  }

  #[inline(always)]
  fn get_partition(&self, key: &K) -> usize {
    (self.random_state.hash_one(key) % P as u64) as usize
  }

  /// Reads N values from the map, leaking only `N` and `B`, but not any information about the keys (doesn't leak the number of keys to each partition).
  /// # Preconditions
  /// * No repeated keys in the input array.
  pub fn get_batch_distinct<const N: usize, const B: usize>(
    &mut self,
    keys: &[K; N],
  ) -> [OOption<V>; N] {
    // 1. Create P arrays of size N.
    let mut partitions: [[BatchBlock<K, V>; N]; P] =
      [unsafe { std::mem::MaybeUninit::<[BatchBlock<K, V>; N]>::uninit().assume_init() }; P];
    const INVALID_ID: usize = usize::max_value();

    // 2. Map each key at index i to a partition: to p[h(keys[i])][i],
    // UNDONE(): this is O(P*N), we could do N log^2 N
    for (i, k) in keys.iter().enumerate() {
      let target_p = self.get_partition(k);
      for (p, partition) in partitions.iter_mut().enumerate() {
        partition[i].k = *k;
        partition[i].index = i;
        partition[i].index.cmov(&INVALID_ID, target_p != p);
      }
    }

    // 3. Apply oblivious compaction to each partition.
    for (p, partition) in partitions.iter_mut().enumerate() {
      let cnt = compact(partition, |x| x.index == INVALID_ID);
      // UNDONE(): deal with overflow.
      assert!(cnt <= B);
    }

    // 4. Read the first B values from each partition in the corresponding partition.
    partitions.par_iter_mut().zip(self.partitions.par_iter_mut()).for_each(|(queries, map)| {
      for i in 0..B {
        let block = &mut queries[i];
        block.v = OOption::new(Default::default(), true);
        block.v.is_some = map.get(block.k, &mut block.v.value);
      }
    });

    // 5. Accumulate the first B values from each partition into the results array.
    let mut merged: Vec<BatchBlock<K, V>> = vec![BatchBlock::default(); P * B];

    for p in 0..P {
      for b in 0..B {
        merged[p * B + b] = partitions[p][b];
      }
    }

    // 6. Oblivious sort according to the index (we actually have P sorted arrays already, so we just need to merge them).
    bitonic_sort(&mut merged);

    // 7. Return the first N values from the results array.
    let mut ret: [OOption<V>; N] = [(); N].map(|_| OOption::default());
    for i in 0..N {
      ret[i] = merged[i].v;
    }

    ret
  }

  /// Inserts a batch of N distinct key-value pairs into the map, distributing them across partitions.
  ///
  /// # Preconditions
  /// * No repeated keys in the input array.
  /// * All of the inserted keys are not already present in the map.
  pub fn insert_batch_distinct<const N: usize, const B: usize>(
    &mut self,
    keys: &[K; N],
    values: &[V; N],
  ) {
    // 1. Create P arrays of size N.
    let mut partitions: [[BatchBlock<K, V>; N]; P] =
      [unsafe { std::mem::MaybeUninit::<[BatchBlock<K, V>; N]>::uninit().assume_init() }; P];
    const INVALID_ID: usize = usize::max_value();

    // 2. Map each key at index i to a partition: to p[h(keys[i])][i],
    // UNDONE(): this is O(P*N), we could do N log^2 N
    for (i, k) in keys.iter().enumerate() {
      let target_p = self.get_partition(k);
      for (p, partition) in partitions.iter_mut().enumerate() {
        partition[i].k = *k;
        partition[i].v = OOption::new(values[i], true);
        partition[i].index = i;
        partition[i].index.cmov(&INVALID_ID, target_p != p);
      }
    }

    // 3. Apply oblivious compaction to each partition.
    for (p, partition) in partitions.iter_mut().enumerate() {
      let cnt = compact(partition, |x| x.index == INVALID_ID);
      // UNDONE(): deal with overflow.
      assert!(cnt <= B);
    }

    // 4. Insert the first B values from each partition in the corresponding partition.
    partitions.par_iter_mut().zip(self.partitions.par_iter_mut()).for_each(|(queries, map)| {
      for i in 0..B {
        let block = &mut queries[i];
        let mut insertion_key = block.k;
        insertion_key.cmov(&K::default(), block.index == INVALID_ID);
        map.insert(insertion_key, block.v.unwrap());
      }
    });

    // 5. Update the size of the map.
    self.size += N;
  }
}


#[cfg(test)]
mod tests {
  use super::*;

  // For all the tests below we keep B == N so that
  // the per‑partition overflow assert! in the map never fires.
  const N: usize = 4;
  const B: usize = N;

  #[test]
  fn new_map_rounds_capacity_and_starts_empty() {
    let requested = 100;
    let map: ShardedMap<u64, u64> = ShardedMap::new(requested);

    // Inside the same module we can see private fields.
    let per_part = (requested + P - 1) / P;
    assert_eq!(map.capacity, per_part * P); // rounded up
    assert_eq!(map.size, 0);
  }

  #[test]
  fn insert_batch_then_get_batch_returns_expected_values() {
    let mut map: ShardedMap<u64, u64> = ShardedMap::new(32);

    let keys:   [u64; N] = [1, 2, 3, 4];
    let values: [u64; N] = [10, 20, 30, 40];

    map.insert_batch_distinct::<N, B>(&keys, &values);

    let results = map.get_batch_distinct::<N, B>(&keys);
    for i in 0..N {
      assert!(results[i].is_some(), "key {} missing", keys[i]);
      assert_eq!(results[i].unwrap(), values[i]);
    }
  }

  #[test]
  fn querying_absent_keys_returns_none() {
    let mut map: ShardedMap<u64, u64> = ShardedMap::new(16);

    let absent: [u64; N] = [100, 200, 300, 400];
    let results = map.get_batch_distinct::<N, B>(&absent);

    for r in &results {
      assert!(r.is_none());
    }
  }

  #[test]
  fn size_updates_after_insert() {
    let mut map: ShardedMap<u64, u64> = ShardedMap::new(16);

    let keys:   [u64; N] = [11, 22, 33, 44];
    let values: [u64; N] = [111, 222, 333, 444];

    map.insert_batch_distinct::<N, B>(&keys, &values);
    assert_eq!(map.size, N);
  }
}