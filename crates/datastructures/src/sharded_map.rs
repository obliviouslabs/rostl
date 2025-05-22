//! Implements map related data structures.

use ahash::RandomState;
use bytemuck::{Pod, Zeroable};
use rods_primitives::{
  cmov_body, cxchg_body, impl_cmov_for_generic_pod,
  traits::{Cmov, _Cmovbase},
};
use rods_sort::compaction::compact;

use crate::map::{OHash, UnsortedMap};

// Size of the insertion queue for deamortized insertions that failed.
const INSERTION_QUEUE_MAX_SIZE: usize = 20;
// Number of deamortized insertions to perform per insertion.
const DEAMORTIZED_INSERTIONS: usize = 2;
// Number of elements in each map bucket.
const BUCKET_SIZE: usize = 2;

use std::mem::MaybeUninit;

/// A shardad hashmap implementation.
/// The map is split across multiple partitions and each partition is a separate hashmap.
/// Queries are resolved in batches, to not leak the number of queries that go to each partition.
/// # Parameters
/// * `K`: The type of the keys in the map.
/// * `V`: The type of the values in the map.
/// * `P`: The number of partitions in the map.
/// * `B`: The maximum number of non-distinct keys in any partition in a batch.
#[derive(Debug)]
pub struct ShardedMap<K, V, const P: usize>
where
  K: OHash + Pod + Default + std::fmt::Debug,
  V: Cmov + Pod + Default + std::fmt::Debug,
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
#[derive(Default, Debug, Clone, Copy, Zeroable)]
struct BatchBlock<K, V>
where
  K: OHash + Pod + Default + std::fmt::Debug,
  V: Cmov + Pod + Default + std::fmt::Debug,
{
  k: K,
  v: V,
  index: usize,
}
unsafe impl<K, V> Pod for BatchBlock<K, V>
where
  K: OHash + Pod + Default + std::fmt::Debug,
  V: Cmov + Pod + Default + std::fmt::Debug,
{
}
impl_cmov_for_generic_pod!(BatchBlock<K, V>; where K: OHash + Pod + Default + std::fmt::Debug, V: Cmov + Pod + Default + std::fmt::Debug);

impl<K, V, const P: usize> ShardedMap<K, V, P>
where
  K: OHash + Default + std::fmt::Debug,
  V: Cmov + Pod + Default + std::fmt::Debug,
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
  pub fn get_batch<const N: usize, const B: usize>(&self, keys: &[K; N]) -> [Option<V>; N] {
    // 1. Create P arrays of size N.
    let mut partitions: [[BatchBlock<K, V>; N]; P] =
      [unsafe { std::mem::MaybeUninit::<[BatchBlock<K, V>; N]>::uninit().assume_init() }; P];

    // 2. Map each key at index i to a partition: to p[h(keys[i])][i],
    for (i, k) in keys.iter().enumerate() {
      let target_p = self.get_partition(k);
      for (p, partition) in partitions.iter_mut().enumerate() {
        partition[i].k = *k;
        partition[i].index = i;
        partition[i].index.cmov(&usize::max_value(), target_p != p);
      }
    }

    // 3. Apply oblivious compaction to each parition.``
    // UNDONE(): auto generated code
    for (p, partition) in partitions.iter_mut().enumerate() {
      let mut dummy = BatchBlock::<K, V>::default();
      let mut dummy_count = 0;
      let mut count = 0;
      for i in 0..N {
        if partition[i].k == dummy.k {
          dummy_count += 1;
        } else {
          partition[count] = partition[i];
          count += 1;
        }
      }
      // Compact the array
      let ret = compact(partition, |x| x.k == dummy.k);
      // ret is the number of elements in the compacted array.
      // We need to set the rest of the array to dummy.
      for i in 0..N {
        let set_dummy = i >= ret;
        partition[i].k.cmov(&dummy.k, set_dummy);
      }
    }

    // 4. Read the first B values from each partition in the corresponding partition.

    // 5. Oblivious sort according to the index (we actually have P sorted arrays already, so we just need to merge them).

    // 6. Return the results array.
    let ret = [None; N];
    ret
  }
}
