//! Implements map related data structures.

use ahash::RandomState;
use bytemuck::{Pod, Zeroable};
use rostl_primitives::{
  cmov_body, cxchg_body, impl_cmov_for_generic_pod,
  ooption::OOption,
  traits::{Cmov, _Cmovbase},
};
use rostl_sort::{
  bitonic::{bitonic_payload_sort, bitonic_sort},
  compaction::{compact, compact_payload, distribute_payload},
};

use crate::map::{OHash, UnsortedMap};
use kanal::{bounded, unbounded, Receiver, Sender};
// use crossbeam::channel::{bounded, unbounded, Receiver, Sender};
use std::{
  io,
  sync::{Arc, Barrier},
  thread,
};
// use tracing::info;

/// Number of partitions in the map.
const P: usize = 15;

/// The replies from the worker thread to the main thread.
enum Reply<K, V>
where
  K: OHash + Pod + Default + std::fmt::Debug + Ord,
  V: Cmov + Pod + Default + std::fmt::Debug,
  BatchBlock<K, V>: Ord + Send,
{
  Blocks { pid: usize, blocks: Vec<BatchBlock<K, V>> },
  Unit(()),
}

enum Replyv2<V>
where
  V: Cmov + Pod + Default + std::fmt::Debug,
{
  Blocks { pid: usize, offset: usize, values: Vec<OOption<V>> },
  Unit(()),
}

/// The command sent to the worker thread to perform a batch operation.
///
enum Cmd<K, V>
where
  K: OHash + Pod + Default + std::fmt::Debug + Ord,
  V: Cmov + Pod + Default + std::fmt::Debug + Eq,
  BatchBlock<K, V>: Ord + Send,
{
  /// Get a batch of blocks from the map.
  Get {
    blocks: Vec<BatchBlock<K, V>>,
    ret_tx: Sender<Reply<K, V>>,
  },
  /// Insert a batch of blocks into the map.
  Insert {
    blocks: Vec<BatchBlock<K, V>>,
    ret_tx: Sender<Reply<K, V>>,
  },
  Getv2 {
    offset: usize,
    blocks: Vec<K>,
  },
  Insertv2 {
    blocks: Vec<KeyWithPartValue<K, V>>,
  },
  // Shutdown the worker thread.
  Shutdown,
}

/// A worker is the thread that manages a partition of the map.
/// Worker threads are kept hot while while there are new queries to process.
#[derive(Debug)]
struct Worker<K, V>
where
  K: OHash + Pod + Default + std::fmt::Debug + Ord,
  V: Cmov + Pod + Default + std::fmt::Debug + Eq,
  BatchBlock<K, V>: Ord + Send,
{
  tx: Sender<Cmd<K, V>>,
  join_handle: Option<thread::JoinHandle<()>>,
}

#[allow(unused)]
fn pin_current_thread_to(cpu: usize) -> io::Result<()> {
  unsafe {
    let mut set: libc::cpu_set_t = std::mem::zeroed();
    libc::CPU_ZERO(&mut set);
    libc::CPU_SET(cpu, &mut set);
    let ret = libc::pthread_setaffinity_np(
      libc::pthread_self(),
      std::mem::size_of::<libc::cpu_set_t>(),
      &raw const set,
    );
    if ret != 0 {
      return Err(io::Error::from_raw_os_error(ret));
    }
  }
  Ok(())
}

fn set_current_thread_rt(priority: i32) -> io::Result<()> {
  unsafe {
    // Check range with sched_get_priority_min/max(SCHED_FIFO)
    let ret = libc::setpriority(libc::PRIO_PROCESS, 0, priority);
    if ret != 0 {
      eprintln!("setpriority failed: {}", std::io::Error::last_os_error());
    }
  }
  Ok(())
}

impl<K, V> Worker<K, V>
where
  K: OHash + Pod + Default + std::fmt::Debug + Ord + Send,
  V: Cmov + Pod + Default + std::fmt::Debug + Send + Eq,
  BatchBlock<K, V>: Ord + Send,
{
  /// Creates a new worker partition `pid`, with max size `n`.
  ///
  fn new(
    n: usize,
    pid: usize,
    startup_barrier: Arc<Barrier>,
    reply_channel: Sender<Replyv2<V>>,
  ) -> Self {
    let (tx, rx): (Sender<Cmd<_, _>>, Receiver<_>) = unbounded();

    let handler = thread::Builder::new()
      .name(format!("partition-{pid}"))
      .spawn(move || {
        // pin thread to CPU
        // pin_current_thread_to(pid).expect("failed to pin thread to CPU");
        set_current_thread_rt(0).expect("failed to set thread to real-time priority");

        // block until all workers are running
        startup_barrier.wait();

        // Thread local variables:
        // Thread-local map for this worker:
        //
        let mut map = UnsortedMap::<K, V>::new(n);

        loop {
          let cmd = match rx.recv() {
            Ok(cmd) => cmd,
            Err(_) => {
              panic!("worker thread command channel disconnected unexpectedly");
            }
          };

          match cmd {
            Cmd::Get { mut blocks, ret_tx } => {
              for blk in &mut blocks {
                blk.v = OOption::new(Default::default(), true);
                blk.v.is_some = map.get(blk.k, &mut blk.v.value);
              }
              let _ = ret_tx.send(Reply::Blocks { pid, blocks }); // move blocks back
            }
            Cmd::Insert { blocks, ret_tx } => {
              for blk in &blocks {
                map.insert_cond(blk.k, blk.v.value, blk.v.is_some);
              }
              let _ = ret_tx.send(Reply::Unit(()));
            }
            Cmd::Getv2 { offset, blocks } => {
              let mut values = vec![OOption::<V>::default(); blocks.len()];
              for (i, k) in blocks.iter().enumerate() {
                values[i].is_some = map.get(*k, &mut values[i].value);
              }
              let _ = reply_channel.send(Replyv2::Blocks { pid, offset, values });
              // move blocks back
            }
            Cmd::Insertv2 { blocks } => {
              for blk in &blocks {
                let real = blk.partition == pid;
                map.insert_cond(blk.key, blk.value, real);
              }
              let _ = reply_channel.send(Replyv2::Unit(()));
            }
            Cmd::Shutdown => break,
          }
        }
      })
      .expect("failed to spawn worker thread");

    Self { tx, join_handle: Some(handler) }
  }
}

impl<K, V> Drop for Worker<K, V>
where
  K: OHash + Pod + Default + std::fmt::Debug + Ord,
  V: Cmov + Pod + Default + std::fmt::Debug + Eq,
  BatchBlock<K, V>: Ord + Send,
{
  fn drop(&mut self) {
    // Send a shutdown command to the worker thread.
    let _ = self.tx.send(Cmd::Shutdown);
    // Wait for the worker thread to finish.
    match self.join_handle.take() {
      Some(handle) => {
        let _ = handle.join();
      }
      None => {
        panic!("Exception while dropping worker thread, handler was already taken");
      }
    }
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
pub struct ShardedMap<K, V>
where
  K: OHash + Pod + Default + std::fmt::Debug + Send + Ord,
  V: Cmov + Pod + Default + std::fmt::Debug + Send + Eq,
  BatchBlock<K, V>: Ord + Send,
{
  /// Number of elements in the map
  size: usize,
  /// capacity
  capacity: usize,
  /// The partitions of the map.
  workers: [Worker<K, V>; P],
  /// The random state used for hashing.
  random_state: RandomState,
  /// Channel for quickly receiving replies from worker threads.
  response_channel: Receiver<Replyv2<V>>,
}

/// A block in a batch, that contains the key, the value and the index of the block in the original full batch.
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

#[repr(C)]
#[derive(Debug, Clone, Copy, Zeroable, PartialEq, Eq)]
struct KeyWithPart<K>
where
  K: OHash + Pod + Default + std::fmt::Debug + Ord + Sized,
{
  partition: usize,
  key: K,
}
unsafe impl<K> Pod for KeyWithPart<K> where K: OHash + Pod + Default + std::fmt::Debug + Ord + Sized {}
impl_cmov_for_generic_pod!(KeyWithPart<K>; where K: OHash + Pod + Default + std::fmt::Debug + Ord + Sized);

impl<K> KeyWithPart<K>
where
  K: OHash + Pod + Default + std::fmt::Debug + Ord + Sized,
{
  fn cmp_ct(&self, other: &Self) -> std::cmp::Ordering {
    let part = self.partition.cmp(&other.partition) as i8;
    let key = self.key.cmp(&other.key) as i8;

    let mut res = part;
    res.cmov(&key, part == 0);

    res.cmp(&0)
  }
}

impl<K> PartialOrd for KeyWithPart<K>
where
  K: OHash + Pod + Default + std::fmt::Debug + Ord + Sized,
{
  #[allow(clippy::non_canonical_partial_ord_impl)]
  fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
    Some(self.cmp_ct(other))
  }
}

impl<K> Ord for KeyWithPart<K>
where
  K: OHash + Pod + Default + std::fmt::Debug + Ord + Sized,
{
  #[allow(clippy::non_canonical_partial_ord_impl)]
  fn cmp(&self, other: &Self) -> std::cmp::Ordering {
    self.cmp_ct(other)
  }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Zeroable, PartialEq, Eq)]
struct KeyWithPartValue<K, V>
where
  K: OHash + Pod + Default + std::fmt::Debug + Ord + Sized,
  V: Cmov + Pod + Default + std::fmt::Debug + Eq,
{
  partition: usize,
  key: K,
  value: V,
}
unsafe impl<K, V> Pod for KeyWithPartValue<K, V>
where
  K: OHash + Pod + Default + std::fmt::Debug + Ord + Sized,
  V: Cmov + Pod + Default + std::fmt::Debug + Eq,
{
}
impl_cmov_for_generic_pod!(KeyWithPartValue<K, V>; where K: OHash + Pod + Default + std::fmt::Debug + Ord + Sized, V: Cmov + Pod + Default + std::fmt::Debug +Eq);

impl<K, V> KeyWithPartValue<K, V>
where
  K: OHash + Pod + Default + std::fmt::Debug + Ord + Sized,
  V: Cmov + Pod + Default + std::fmt::Debug + Eq,
{
  fn cmp_ct(&self, other: &Self) -> std::cmp::Ordering {
    let part = self.partition.cmp(&other.partition) as i8;
    let key = self.key.cmp(&other.key) as i8;

    let mut res = part;
    res.cmov(&key, part == 0);

    res.cmp(&0)
  }
}

impl<K, V> PartialOrd for KeyWithPartValue<K, V>
where
  K: OHash + Pod + Default + std::fmt::Debug + Ord + Sized,
  V: Cmov + Pod + Default + std::fmt::Debug + Eq,
{
  #[allow(clippy::non_canonical_partial_ord_impl)]
  fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
    Some(self.cmp_ct(other))
  }
}

impl<K, V> Ord for KeyWithPartValue<K, V>
where
  K: OHash + Pod + Default + std::fmt::Debug + Ord + Sized,
  V: Cmov + Pod + Default + std::fmt::Debug + Eq,
{
  fn cmp(&self, other: &Self) -> std::cmp::Ordering {
    self.cmp_ct(other)
  }
}

impl<K, V> ShardedMap<K, V>
where
  K: OHash + Default + std::fmt::Debug + Send + Ord + Pod + Sized,
  V: Cmov + Pod + Default + std::fmt::Debug + Send + Eq,
  BatchBlock<K, V>: Ord + Send,
{
  /// Creates a new `ShardedMap` with the given number of partitions.
  pub fn new(capacity: usize) -> Self {
    let per_part = capacity.div_ceil(P);
    let startup = Arc::new(Barrier::new(P + 1));

    let (reply_tx, response_channel) = unbounded::<Replyv2<V>>();

    let workers =
      std::array::from_fn(|i| Worker::new(per_part, i, startup.clone(), reply_tx.clone()));

    // wait until all workers have reached their barrier
    startup.wait();

    Self {
      size: 0,
      capacity: per_part * P,
      workers,
      random_state: RandomState::new(),
      response_channel,
    }
  }

  #[inline(always)]
  fn get_partition(&self, key: &K) -> usize {
    (self.random_state.hash_one(key) % P as u64) as usize
  }

  /// Computes a safe batch size B for a given number of distinct queries N.
  /// # Preconditions
  /// * N >= P log P
  pub const fn compute_safe_batch_size(&self, n: usize) -> usize {
    let a = n.div_ceil(P) + (n * (P.ilog2() as usize + 1)).div_ceil(P).isqrt() + 20; // Safety margin for small N
    if a < n {
      a
    } else {
      n
    }
  }

  /// Reads N values from the map, leaking only `N` and `B`, but not any information about the keys (doesn't leak the number of keys to each partition).
  /// # Preconditions
  /// * No repeated keys in the input array.
  /// * There are at most `b` queries to each partition.
  pub fn get_batch_distinct(&mut self, keys: &[K], b: usize) -> Vec<OOption<V>> {
    let n: usize = keys.len();
    assert!(b <= n, "batch size b must be <= number of keys");

    // 1. Create P arrays of size N.
    // let mut per_p: [[BatchBlock<K, V>; N]; P] =
    //   [unsafe { std::mem::MaybeUninit::<[BatchBlock<K, V>; N]>::uninit().assume_init() }; P];
    let mut per_p: [Box<Vec<BatchBlock<K, V>>>; P] =
      std::array::from_fn(|_| Box::new(vec![BatchBlock::default(); n]));

    const INVALID_ID: usize = usize::MAX;

    // 2. Map each key at index i to a partition: to p[h(keys[i])][i],
    // UNDONE(git-65): this is O(P*n), we could do n log^2 n
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
      let cnt = compact(partition, |x: &BatchBlock<K, V>| x.index == INVALID_ID);
      // UNDONE(git-64): deal with overflow.
      assert!(cnt <= b);
    }

    let (done_tx, done_rx) = bounded::<Reply<K, V>>(P);

    // 4. Read the first B values from each partition in the corresponding partition.
    for (p, partition) in per_p.iter_mut().enumerate() {
      let blocks: Vec<BatchBlock<K, V>> = partition[..b].to_vec();
      self.workers[p].tx.send(Cmd::Get { blocks, ret_tx: done_tx.clone() }).unwrap();
    }

    // 5. Collect the first B values from each partition into the results array.
    let mut merged: Vec<BatchBlock<K, V>> = vec![BatchBlock::default(); P * b];

    for _ in 0..P {
      match done_rx.recv().unwrap() {
        Reply::Blocks { pid, blocks } => {
          for i in 0..b {
            merged[pid * b + i] = blocks[i];
          }
        }
        _ => panic!("unexpected reply from worker thread (probably early termination?)"),
      }
    }

    // 6. Oblivious sort according to the index (we actually have P sorted arrays already, so we just need to merge them).
    bitonic_sort(&mut merged);

    // 7. Return the first n values from the results array.
    let mut ret: Vec<OOption<V>> = vec![OOption::default(); n];

    for i in 0..n {
      ret[i] = merged[i].v;
    }

    ret
  }

  /// Reads N values from the map, leaking only `N` and `B`, but not any information about the keys (doesn't leak the number of keys to each partition).
  /// # Preconditions
  /// * There are at most `b` queries to each partition (statistically likely).
  pub fn get_batch(&mut self, keys: &[K], b: usize) -> Vec<OOption<V>> {
    // info!("get_batch called with n = {}, b = {}", keys.len(), b);
    // let now = std::time::Instant::now();
    let n: usize = keys.len();
    assert!(n > 0, "get_batch requires at least one key");
    assert!(b <= n, "batch size b must be <= number of keys");
    let bp = b * P;
    assert!(b >= n.div_ceil(P), "batch size b must be >= n/P to avoid overflow");
    const SUBTASK_SIZE: usize = 32;

    // 1. Sort the keys by partition.
    let mut keyinfo = vec![KeyWithPart { partition: P, key: K::default() }; bp];
    for i in 0..n {
      keyinfo[i].key = keys[i];
      keyinfo[i].partition = self.get_partition(&keys[i]);
    }

    let mut index_map_1 = (0..n).collect::<Vec<usize>>();
    bitonic_payload_sort::<KeyWithPart<K>, [KeyWithPart<K>], usize>(
      &mut keyinfo[..n],
      &mut index_map_1,
    );

    // 2. Compute unique keys for each partition and remove duplicates to the end.
    let mut par_load = [0; P];
    let mut prefix_sum_1 = vec![0; n + 1];

    prefix_sum_1[1] = 1;
    for (j, load) in par_load.iter_mut().enumerate() {
      let cond = keyinfo[0].partition == j;
      load.cmov(&1, cond);
    }
    for i in 1..n {
      let new_key = keyinfo[i].key != keyinfo[i - 1].key;
      prefix_sum_1[i + 1] = prefix_sum_1[i];

      let alt = prefix_sum_1[i] + 1;
      prefix_sum_1[i + 1].cmov(&alt, new_key);

      for (j, load) in par_load.iter_mut().enumerate() {
        let cond = keyinfo[i].partition == j;
        let alt = *load + 1;
        load.cmov(&alt, cond & new_key);
      }
    }
    // for i in n..(np + 1) {
    //   prefix_sum_1[i] = prefix_sum_1[n];
    // }

    for (j, load) in par_load.iter().enumerate() {
      assert!(*load <= b, "Too many distinct keys in partition {j}: {}, increase b", *load);
    }

    compact_payload(&mut keyinfo[..n], &prefix_sum_1);

    // 3. Create a distribution of the unique keys to partitions.
    let mut par_load_ps = [0; P + 1];
    for j in 0..P {
      par_load_ps[j + 1] = par_load_ps[j] + par_load[j];
    }
    let mut prefix_sum_2 = vec![0; bp + 1];
    for j in 0..P {
      for i in 0..b {
        let mut rank_in_part = i + 1;
        rank_in_part.cmov(&par_load[j], rank_in_part > par_load[j]);
        prefix_sum_2[j * b + i + 1] = par_load_ps[j] + rank_in_part;
      }
    }
    distribute_payload(&mut keyinfo, &prefix_sum_2);

    // info!("get_batch preprocessing took {:?}", now.elapsed());
    // let now = std::time::Instant::now();

    let mut sent_count = 0;
    // 4. Read the first B values from each partition in the corresponding partition.
    for j in 0..P {
      for k in 0..b.div_ceil(SUBTASK_SIZE) {
        let offset = k * SUBTASK_SIZE;
        let low = j * b + offset;
        let high = (low + SUBTASK_SIZE).min((j + 1) * b);
        let blocks: Vec<K> = keyinfo[low..high].iter().map(|x| x.key).collect();
        self.workers[j].tx.send(Cmd::Getv2 { offset, blocks }).unwrap();
        sent_count += 1;
      }
    }

    let mut res = vec![OOption::<V>::default(); bp];

    for _ in 0..sent_count {
      match self.response_channel.recv().unwrap() {
        Replyv2::Blocks { pid, offset, values } => {
          for (val, res) in
            values.iter().zip(res.iter_mut().skip(pid * b + offset)).take(SUBTASK_SIZE)
          {
            *res = *val;
          }
        }
        _ => panic!("unexpected reply from worker thread (probably early termination?)"),
      }
    }
    // info!("get_batch querying took {:?}", now.elapsed());
    // let now = std::time::Instant::now();

    // 5. Undo compaction and distribution of the results.
    compact_payload(&mut res, &prefix_sum_2);
    distribute_payload(&mut res[..n], &prefix_sum_1);

    for i in 1..n {
      let cond = prefix_sum_1[i] == prefix_sum_1[i - 1];
      let copy = res[i - 1];
      res[i].cmov(&copy, cond);
    }

    res.truncate(n);
    bitonic_payload_sort(&mut index_map_1[..n], &mut res);

    // info!("get_batch postprocessing took {:?}", now.elapsed());

    res
  }

  /// Leaky version of `get_batch_distinct`, which will return values for repeated keys and leak the size of the largest partition.
  /// # Safety
  /// * This function will leak the size of the largest partition, which with repeated queries can be used to infer the mapping of keys to partitions.
  #[deprecated(
    note = "This function is unsafe because it can potentially leak information about keys to partition mapping. Use get_batch_distinct instead."
  )]
  pub unsafe fn get_batch_leaky(&mut self, keys: &[K]) -> Vec<OOption<V>> {
    let n: usize = keys.len();
    let mut b = 0;

    // 1. Create P arrays of size N.
    // let mut per_p: [[BatchBlock<K, V>; N]; P] =
    //   [unsafe { std::mem::MaybeUninit::<[BatchBlock<K, V>; N]>::uninit().assume_init() }; P];
    let mut per_p: [Box<Vec<BatchBlock<K, V>>>; P] =
      std::array::from_fn(|_| Box::new(vec![BatchBlock::default(); n]));

    const INVALID_ID: usize = usize::MAX;

    // 2. Map each key at index i to a partition: to p[h(keys[i])][i],
    // UNDONE(git-65): this is O(P*n), we could do n log^2 n
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
      let cnt = compact(partition, |x: &BatchBlock<K, V>| x.index == INVALID_ID);
      b = b.max(cnt);
    }

    let (done_tx, done_rx) = bounded::<Reply<K, V>>(P);

    // 4. Read the first B values from each partition in the corresponding partition.
    for (p, partition) in per_p.iter_mut().enumerate() {
      let blocks: Vec<BatchBlock<K, V>> = partition[..b].to_vec();
      self.workers[p].tx.send(Cmd::Get { blocks, ret_tx: done_tx.clone() }).unwrap();
    }

    // 5. Collect the first B values from each partition into the results array.
    let mut merged: Vec<BatchBlock<K, V>> = vec![BatchBlock::default(); P * b];

    for _ in 0..P {
      match done_rx.recv().unwrap() {
        Reply::Blocks { pid, blocks } => {
          for i in 0..b {
            merged[pid * b + i] = blocks[i];
          }
        }
        _ => panic!("unexpected reply from worker thread (probably early termination?)"),
      }
    }

    // 6. Oblivious sort according to the index (we actually have P sorted arrays already, so we just need to merge them).
    bitonic_sort(&mut merged);

    // 7. Return the first n values from the results array.
    let mut ret: Vec<OOption<V>> = vec![OOption::default(); n];

    for i in 0..n {
      ret[i] = merged[i].v;
    }

    ret
  }

  /// Inserts a batch of N distinct key-value pairs into the map, distributing them across partitions.
  ///
  /// # Preconditions
  /// * No repeated keys in the input array.
  /// * All of the inserted keys are not already present in the map.
  /// * There is enough space in the map to insert all `N` keys.
  /// * There are at most `b` queries to each partition.
  pub fn insert_batch_distinct(&mut self, keys: &[K], values: &[V], b: usize) {
    let n = keys.len();
    assert!(n == values.len(), "Invalid input: keys and values must have the same length");
    assert!(self.size + n <= self.capacity, "Map is full, cannot insert more elements.");
    assert!(b <= n, "batch size b must be <= number of keys");

    // 1. Create P arrays of size N.
    let mut per_p: [Box<Vec<BatchBlock<K, V>>>; P] =
      std::array::from_fn(|_| Box::new(vec![BatchBlock::default(); n]));

    const INVALID_ID: usize = usize::MAX;

    // 2. Map each key at index i to a partition: to p[h(keys[i])][i],
    // UNDONE(git-65): this is O(P*N), we could do N log^2 N
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
      let cnt = compact(partition, |x| x.index == INVALID_ID);

      // UNDONE(git-64): deal with overflow.
      assert!(cnt <= b);
    }

    let (done_tx, done_rx) = bounded::<Reply<K, V>>(P);

    // 4. Insert the first b values from each partition in the corresponding partition.
    for (p, partition) in per_p.iter_mut().enumerate() {
      let blocks: Vec<BatchBlock<K, V>> = partition[..b].to_vec();
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
    self.size += n;
  }

  /// Reads N values from the map, leaking only `N` and `B`, but not any information about the keys (doesn't leak the number of keys to each partition).
  /// # Preconditions
  /// * There are at most `b` queries to each partition (statistically likely).
  /// # Behavior
  /// * If a key appears multiple times in the input array, only the value corresponding to its first occurrence is used.
  pub fn insert_batch(&mut self, keys: &[K], values: &[V], b: usize) {
    let n: usize = keys.len();
    assert!(n > 0, "get_batch requires at least one key");
    assert!(b <= n, "batch size b must be <= number of keys");
    let bp = b * P;
    assert!(b >= n.div_ceil(P), "batch size b must be >= n/P to avoid overflow");

    // 1. Sort the keys by partition.
    let mut keyinfo =
      vec![KeyWithPartValue { partition: P, key: K::default(), value: V::default() }; bp];
    for i in 0..n {
      keyinfo[i].key = keys[i];
      keyinfo[i].value = values[i];
      keyinfo[i].partition = self.get_partition(&keys[i]);
    }

    let mut index_map_1 = (0..n).collect::<Vec<usize>>();
    bitonic_payload_sort::<KeyWithPartValue<K, V>, [KeyWithPartValue<K, V>], usize>(
      &mut keyinfo[..n],
      &mut index_map_1,
    );

    // 2. Compute unique keys for each partition and remove duplicates to the end.
    let mut par_load = [0; P];
    let mut prefix_sum_1 = vec![0; n + 1];

    prefix_sum_1[1] = 1;
    for (j, load) in par_load.iter_mut().enumerate() {
      let cond = keyinfo[0].partition == j;
      load.cmov(&1, cond);
    }
    for i in 1..n {
      let new_key = keyinfo[i].key != keyinfo[i - 1].key;
      prefix_sum_1[i + 1] = prefix_sum_1[i];

      let alt = prefix_sum_1[i] + 1;
      prefix_sum_1[i + 1].cmov(&alt, new_key);
      keyinfo[i].partition.cmov(&P, !new_key); // Mark duplicate keys as belonging to an invalid partition

      for (j, load) in par_load.iter_mut().enumerate() {
        let cond = keyinfo[i].partition == j;
        let alt = *load + 1;
        load.cmov(&alt, cond & new_key);
      }
    }
    // for i in n..(np + 1) {
    //   prefix_sum_1[i] = prefix_sum_1[n];
    // }

    for (j, load) in par_load.iter().enumerate() {
      assert!(*load <= b, "Too many distinct keys in partition {j}: {}, increase b", *load);
    }

    compact_payload(&mut keyinfo[..n], &prefix_sum_1);

    // 3. Create a distribution of the unique keys to partitions.
    let mut par_load_ps = [0; P + 1];
    for j in 0..P {
      par_load_ps[j + 1] = par_load_ps[j] + par_load[j];
    }
    self.size += par_load_ps[P];

    let mut prefix_sum_2 = vec![0; bp + 1];
    for j in 0..P {
      for i in 0..b {
        let mut rank_in_part = i + 1;
        rank_in_part.cmov(&par_load[j], rank_in_part > par_load[j]);
        prefix_sum_2[j * b + i + 1] = par_load_ps[j] + rank_in_part;
      }
    }
    distribute_payload(&mut keyinfo, &prefix_sum_2);

    // 4. Read the first B values from each partition in the corresponding partition.
    const SUBTASK_SIZE: usize = 32;
    let mut sent_count = 0;
    for j in 0..P {
      for k in 0..b.div_ceil(SUBTASK_SIZE) {
        let low = j * b + k * SUBTASK_SIZE;
        let high = (low + SUBTASK_SIZE).min((j + 1) * b);
        let blocks: Vec<KeyWithPartValue<K, V>> = keyinfo[low..high].to_vec();
        self.workers[j].tx.send(Cmd::Insertv2 { blocks }).unwrap();
        sent_count += 1;
      }
    }

    for _ in 0..sent_count {
      match self.response_channel.recv().unwrap() {
        Replyv2::Unit(()) => {}
        _ => panic!("unexpected reply from worker thread (probably early termination?)"),
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  // For all the tests below we keep b == N so that
  // the per‑partition overflow assert! in the map never fires.
  const N: usize = 4;

  #[test]
  fn new_map_rounds_capacity_and_starts_empty() {
    let requested = 100;
    let map: ShardedMap<u64, u64> = ShardedMap::new(requested);

    // Inside the same module we can see private fields.
    let per_part = requested.div_ceil(P);
    assert_eq!(map.capacity, per_part * P); // rounded up
    assert_eq!(map.size, 0);
  }

  #[test]
  fn insert_batch_then_get_batch_returns_expected_values() {
    let mut map: ShardedMap<u64, u64> = ShardedMap::new(32);

    let keys: [u64; N] = [1, 2, 3, 4];
    let values: [u64; N] = [10, 20, 30, 40];

    map.insert_batch_distinct(&keys, &values, N);

    let results = map.get_batch_distinct(&keys, N);
    for i in 0..N {
      assert!(results[i].is_some(), "key {} missing", keys[i]);
      assert_eq!(results[i].unwrap(), values[i]);
    }

    #[allow(deprecated)]
    {
      let results = unsafe { map.get_batch_leaky(&keys) };
      for i in 0..N {
        assert!(results[i].is_some(), "key {} missing", keys[i]);
        assert_eq!(results[i].unwrap(), values[i]);
      }
    }

    let results = map.get_batch(&keys, N);
    for i in 0..N {
      assert!(results[i].is_some(), "key {} missing", keys[i]);
      assert_eq!(results[i].unwrap(), values[i]);
    }
  }

  #[test]
  fn querying_absent_keys_returns_none() {
    let mut map: ShardedMap<u64, u64> = ShardedMap::new(16);

    let absent: [u64; N] = [100, 200, 300, 400];
    let results = map.get_batch_distinct(&absent, N);

    for r in &results {
      assert!(!r.is_some());
    }

    #[allow(deprecated)]
    {
      let results = unsafe { map.get_batch_leaky(&absent) };
      for r in &results {
        assert!(!r.is_some());
      }
    }

    let absent: [u64; N] = [100, 200, 300, 400];
    let results = map.get_batch(&absent, N);

    for r in &results {
      assert!(!r.is_some());
    }
  }

  #[test]
  fn size_updates_after_insert() {
    let mut map: ShardedMap<u64, u64> = ShardedMap::new(16);

    let keys: [u64; N] = [11, 22, 33, 44];
    let values: [u64; N] = [111, 222, 333, 444];

    map.insert_batch_distinct(&keys, &values, N);
    assert_eq!(map.size, N);

    let mut map: ShardedMap<u64, u64> = ShardedMap::new(16);
    map.insert_batch(&keys, &values, N);
    assert_eq!(map.size, N);
  }

  #[test]
  fn compute_safe_batch_size_works() {
    let map: ShardedMap<u64, u64> = ShardedMap::new(16);

    // N >= P log P
    assert_eq!(P, 15);

    let n = 100;
    let b = map.compute_safe_batch_size(n);
    assert!(b >= n.div_ceil(P));
    assert_eq!(b, 32);

    let n = 1000;
    let b = map.compute_safe_batch_size(n);
    assert!(b >= n.div_ceil(P));
    assert_eq!(b, 103);

    let n = 4096;
    let b = map.compute_safe_batch_size(n);
    assert!(b >= n.div_ceil(P));
    assert_eq!(b, 327);

    let n = 8192;
    let b = map.compute_safe_batch_size(n);
    assert!(b >= n.div_ceil(P));
    assert_eq!(b, 613);

    for i in 1..100 {
      for j in 1..100 {
        let n = i * j * P;
        let b = map.compute_safe_batch_size(n);
        assert!(b >= n.div_ceil(P));
        assert!(b <= n);
      }
    }
  }
}
