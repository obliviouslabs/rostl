//! Implements map related data structures.

use ahash::RandomState;
use bytemuck::{Pod, Zeroable};
use rods_primitives::{
  cmov_body, cxchg_body, impl_cmov_for_generic_pod,
  ooption::OOption,
  traits::{Cmov, _Cmovbase},
};
use rods_sort::{bitonic::bitonic_sort, compaction::compact};

use crate::map::{OHash, UnsortedMap};
use kanal::{bounded, Receiver, Sender};
use std::{
  sync::{Arc, Barrier},
  thread,
};

// Number of partitions in the map.
const P: usize = 15;

/// The replies from the worker thread to the main thread.
enum Reply<K, V, const B: usize>
where
  K: OHash + Pod + Default + std::fmt::Debug + Ord,
  V: Cmov + Pod + Default + std::fmt::Debug,
  BatchBlock<K, V>: Ord + Send,
{
  Blocks { pid: usize, blocks: Box<[BatchBlock<K, V>; B]> },
  Unit(()),
}

/// The command sent to the worker thread to perform a batch operation.
///
enum Cmd<K, V, const B: usize>
where
  K: OHash + Pod + Default + std::fmt::Debug + Ord,
  V: Cmov + Pod + Default + std::fmt::Debug,
  BatchBlock<K, V>: Ord + Send,
{
  /// Get a batch of blocks from the map.
  Get { blocks: Box<[BatchBlock<K, V>; B]>, ret_tx: Sender<Reply<K, V, B>> },
  /// Insert a batch of blocks into the map.
  Insert { blocks: Box<[BatchBlock<K, V>; B]>, ret_tx: Sender<Reply<K, V, B>> },
  // UNDONE(): Implement shutdown logic.
  // Shutdown,
}

/// A worker is the thread that manages a partition of the map.
/// Worker threads are kept hot while while there are new queries to proccess.
#[derive(Debug)]
struct Worker<K, V, const B: usize>
where
  K: OHash + Pod + Default + std::fmt::Debug + Ord,
  V: Cmov + Pod + Default + std::fmt::Debug,
  BatchBlock<K, V>: Ord + Send,
{
  tx: Sender<Cmd<K, V, B>>,
}

impl<K, V, const B: usize> Worker<K, V, B>
where
  K: OHash + Pod + Default + std::fmt::Debug + Ord + Send,
  V: Cmov + Pod + Default + std::fmt::Debug + Send,
  BatchBlock<K, V>: Ord + Send,
{
  /// Creates a new worker partition `pid`, with max size `n`.
  ///
  #[allow(tail_expr_drop_order)]
  fn new(n: usize, pid: usize, startup_barrier: Arc<Barrier>) -> Self {
    // UNDONE(): this bound is a bit arbitrary, 2 should be enough.
    let (tx, rx): (Sender<Cmd<_, _, B>>, Receiver<_>) = bounded(10);

    thread::Builder::new()
      .name(format!("partition-{pid}"))
      .spawn(move || {
        // block until all workers are running
        startup_barrier.wait();

        // Thread local variables:
        // Thread-local map for this worker:
        //
        let mut map = UnsortedMap::<K, V>::new(n);

        while let Ok(cmd) = rx.recv() {
          match cmd {
            Cmd::Get { mut blocks, ret_tx } => {
              println!("worker {pid} received get command with {} blocks", blocks.len());
              for blk in blocks.iter_mut() {
                blk.v = OOption::new(Default::default(), true);
                blk.v.is_some = map.get(blk.k, &mut blk.v.value);
              }
              let _ = ret_tx.send(Reply::Blocks { pid, blocks }); // move blocks back
            }
            Cmd::Insert { blocks, ret_tx } => {
              println!("worker {pid} received insert command with {} blocks", blocks.len());
              for blk in blocks.iter() {
                map.insert(blk.k, blk.v.unwrap());
              }
              let _ = ret_tx.send(Reply::Unit(()));
            } // UNDONE(): Implement shutdown logic.
              // Cmd::Shutdown => {
              //   // We don't need to do anything here, the worker will exit.
              //   break;
              // }
          }
        }
      })
      .expect("failed to spawn worker thread");

    Self { tx }
  }
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
pub struct ShardedMap<K, V, const B: usize>
where
  K: OHash + Pod + Default + std::fmt::Debug + Send + Ord,
  V: Cmov + Pod + Default + std::fmt::Debug + Send,
  BatchBlock<K, V>: Ord + Send,
{
  /// Number of elements in the map
  size: usize,
  /// capacity
  capacity: usize,
  /// The partitions of the map.
  workers: [Worker<K, V, B>; P],
  /// The random state used for hashing.
  random_state: RandomState,
}

/// A block in a batch, that containts the key, the value and the index of the block in the original full batch.
#[repr(C)]
#[derive(Default, Debug, Clone, Copy, Zeroable, PartialEq, Eq, PartialOrd, Ord)]
pub struct BatchBlock<K, V>
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

impl<K, V, const B: usize> ShardedMap<K, V, B>
where
  K: OHash + Default + std::fmt::Debug + Send + Ord,
  V: Cmov + Pod + Default + std::fmt::Debug + Send,
  BatchBlock<K, V>: Ord + Send,
{
  /// Creates a new `ShardedMap` with the given number of partitions.
  pub fn new(capacity: usize) -> Self {
    let per_part = capacity.div_ceil(P);
    let startup = Arc::new(Barrier::new(P + 1));

    let workers = std::array::from_fn(|i| Worker::new(per_part, i, startup.clone()));

    // wait until all workers have reached their barrier
    startup.wait();

    Self { size: 0, capacity: per_part * P, workers, random_state: RandomState::new() }
  }

  #[inline(always)]
  fn get_partition(&self, key: &K) -> usize {
    (self.random_state.hash_one(key) % P as u64) as usize
  }

  /// Reads N values from the map, leaking only `N` and `B`, but not any information about the keys (doesn't leak the number of keys to each partition).
  /// # Preconditions
  /// * No repeated keys in the input array.
  pub fn get_batch_distinct<const N: usize>(&mut self, keys: &[K; N]) -> [OOption<V>; N] {
    // 1. Create P arrays of size N.
    // let mut per_p: [[BatchBlock<K, V>; N]; P] =
    //   [unsafe { std::mem::MaybeUninit::<[BatchBlock<K, V>; N]>::uninit().assume_init() }; P];
    let mut per_p: [Box<[BatchBlock<K, V>; B]>; P] =
      std::array::from_fn(|_| Box::new([BatchBlock::default(); B]));

    const INVALID_ID: usize = usize::MAX;

    // 2. Map each key at index i to a partition: to p[h(keys[i])][i],
    // UNDONE(): this is O(P*N), we could do N log^2 N
    for (i, k) in keys.iter().enumerate() {
      let target_p = self.get_partition(k);
      for (p, partition) in per_p.iter_mut().enumerate() {
        partition[i].k = *k;
        partition[i].index = i;
        partition[i].index.cmov(&INVALID_ID, target_p != p);
      }
    }

    // 3. Apply oblivious compaction to each partition.
    for partition in &mut per_p {
      let cnt = compact(&mut **partition, |x: &BatchBlock<K, V>| x.index == INVALID_ID);
      // UNDONE(): deal with overflow.
      assert!(cnt <= B);
    }

    let (done_tx, done_rx) = bounded::<Reply<K, V, B>>(P);

    // 4. Read the first B values from each partition in the corresponding partition.
    for (p, partition) in per_p.iter_mut().enumerate() {
      let blocks = std::mem::replace(partition, Box::new([BatchBlock::default(); B]));
      self.workers[p].tx.send(Cmd::Get { blocks, ret_tx: done_tx.clone() }).unwrap();
    }

    // 5. Collect the first B values from each partition into the results array.
    let mut merged: Vec<BatchBlock<K, V>> = vec![BatchBlock::default(); P * B];

    for _ in 0..P {
      match done_rx.recv().unwrap() {
        Reply::Blocks { pid, blocks } => {
          for b in 0..B {
            merged[pid * B + b] = blocks[b];
          }
        }
        _ => {
          panic!("unexpected reply from worker thread (probably early termination?)");
        }
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
  /// * There is enough space in the map to insert all N keys.
  pub fn insert_batch_distinct<const N: usize>(&mut self, keys: &[K; N], values: &[V; N]) {
    assert!(self.size + N <= self.capacity, "Map is full, cannot insert more elements.");
    // 1. Create P arrays of size N.
    let mut per_p: [Box<[BatchBlock<K, V>; B]>; P] =
      std::array::from_fn(|_| Box::new([BatchBlock::default(); B]));

    const INVALID_ID: usize = usize::MAX;

    // 2. Map each key at index i to a partition: to p[h(keys[i])][i],
    // UNDONE(): this is O(P*N), we could do N log^2 N
    for (i, k) in keys.iter().enumerate() {
      let target_p = self.get_partition(k);
      for (p, partition) in per_p.iter_mut().enumerate() {
        partition[i].k = *k;
        partition[i].v = OOption::new(values[i], true);
        partition[i].index = i;
        partition[i].index.cmov(&INVALID_ID, target_p != p);
      }
    }

    // 3. Apply oblivious compaction to each partition.
    for partition in &mut per_p {
      let cnt = compact(&mut **partition, |x| x.index == INVALID_ID);
      // UNDONE(): deal with overflow.
      assert!(cnt <= B);
    }

    let (done_tx, done_rx) = bounded::<Reply<K, V, B>>(P);

    // 4. Insert the first B values from each partition in the corresponding partition.
    for (p, partition) in per_p.iter_mut().enumerate() {
      let blocks = std::mem::replace(partition, Box::new([BatchBlock::default(); B]));
      self.workers[p].tx.send(Cmd::Insert { blocks, ret_tx: done_tx.clone() }).unwrap();
    }

    // 5. Receive the write receipts from the worker threads.
    for _i in 0..P {
      match done_rx.recv().unwrap() {
        Reply::Unit(()) => {}
        _ => {
          panic!("unexpected reply from worker thread (probably early termination?)");
        }
      }
    }

    // 6. Update the size of the map.
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
    let map: ShardedMap<u64, u64, B> = ShardedMap::new(requested);

    // Inside the same module we can see private fields.
    let per_part = requested.div_ceil(P);
    assert_eq!(map.capacity, per_part * P); // rounded up
    assert_eq!(map.size, 0);
  }

  #[test]
  fn insert_batch_then_get_batch_returns_expected_values() {
    let mut map: ShardedMap<u64, u64, B> = ShardedMap::new(32);

    let keys: [u64; N] = [1, 2, 3, 4];
    let values: [u64; N] = [10, 20, 30, 40];

    map.insert_batch_distinct::<N>(&keys, &values);

    let results = map.get_batch_distinct::<N>(&keys);
    for i in 0..N {
      assert!(results[i].is_some(), "key {} missing", keys[i]);
      assert_eq!(results[i].unwrap(), values[i]);
    }
  }

  #[test]
  fn querying_absent_keys_returns_none() {
    let mut map: ShardedMap<u64, u64, B> = ShardedMap::new(16);

    let absent: [u64; N] = [100, 200, 300, 400];
    let results = map.get_batch_distinct::<N>(&absent);

    for r in &results {
      assert!(!r.is_some());
    }
  }

  #[test]
  fn size_updates_after_insert() {
    let mut map: ShardedMap<u64, u64, B> = ShardedMap::new(16);

    let keys: [u64; N] = [11, 22, 33, 44];
    let values: [u64; N] = [111, 222, 333, 444];

    map.insert_batch_distinct::<N>(&keys, &values);
    assert_eq!(map.size, N);
  }
}
