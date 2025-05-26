//! Implements map related data structures.

use ahash::RandomState;
use bytemuck::{Pod, Zeroable};
use rods_primitives::{
  cmov_body, cxchg_body, impl_cmov_for_generic_pod,
  traits::{Cmov, _Cmovbase},
};
use rods_sort::{bitonic::bitonic_sort, compaction::compact};
use rayon::prelude::*;

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

/// A shardad hashmap implementation.
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
  k: K,
  v: V,
  index: usize,
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
  UnsortedMap<K, V>: Send + Sync 
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
  pub fn get_batch<const N: usize, const B: usize>(&mut self, keys: &[K; N]) -> [Option<V>; N] {
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
      debug_assert!(cnt <= B);
    }

    // 4. Read the first B values from each partition in the corresponding partition.
    (&mut partitions[..])
      .par_iter_mut().zip((&mut self.partitions[..]).par_iter_mut()).for_each(
      |(partition, map)| {
        for i in 0..B {
          let block = &mut partition[i];
          let _ret = map.get(block.k, &mut block.v);
          debug_assert!(_ret >= (block.index != INVALID_ID));
        }
      },
    );

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
    // UNDONE(): we are leaking the result of each queried value
    let mut ret: [Option<V>; N] = [None; N];
    for i in 0..N {
      let block = &merged[i];
      if block.index != INVALID_ID {
        ret[block.index] = Some(block.v);
      }
    }

    let ret = [None; N];
    ret
  }
}
