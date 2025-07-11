//! This module implements oblivious queues
#![allow(clippy::needless_bitwise_bool)] // UNDONE(git-8): This is needed to enforce the bitwise operations to not short circuit. Investigate if we should be using helper functions instead.

use bytemuck::{Pod, Zeroable};
use rostl_primitives::{
  cmov_body, cxchg_body, impl_cmov_for_generic_pod, indexable::Length, traits::Cmov,
  traits::_Cmovbase,
};

use crate::array::ShortArray;

/// An element in a short queue.
/// See `ShortQueue` for more details.
/// # Invariant
/// * `timestamp` == 0 ==> `value` is not valid
/// * `timestamp` != 0 ==> `value` is valid and in the queue
/// * `timestamp` is unique for each enqueued element and in the range `[lowest_timestamp, highest_timestamp]`
#[repr(C)]
#[derive(Debug, Default, Clone, Copy, Zeroable)]
pub struct ShortQueueElement<T>
where
  T: Cmov + Pod,
{
  timestamp: usize,
  pub(crate) value: T,
}
unsafe impl<T: Cmov + Pod> Pod for ShortQueueElement<T> {}
impl_cmov_for_generic_pod!(ShortQueueElement<T>; where T: Cmov + Pod);

impl<T> ShortQueueElement<T>
where
  T: Cmov + Pod,
{
  /// Returns true if the element is empty.
  pub const fn is_empty(&self) -> bool {
    self.timestamp == 0
  }
}

/// Implements a queue with a fixed maximum size.
/// The queue access pattern and size are oblivious.
///
/// There are two trivial efficient ways to implement this for short queues:
/// 1. Use oblivious compaction:
///  - Push: n (2 + log n)
///  - Pop: n
///  - Iter: n
/// 2. Use timestamps:
///  - Push: n
///  - Pop: n
///  - Iter: n (1 + log^2 n)
///
/// This implementation uses timestamps (2.), as we only need push and pop for the unsorted map.
/// # Invariants
/// * `highest_timestamp` is the timestamp of the most recently added element
/// * `lowest_timestamp` is the timestamp of the oldest element added, or just non-zero if the queue is empty
/// * `size` is the number of `elements` with non-zero timestamps
/// * an element in `elements` is valid if its timestamp is non-zero, in which case the timestamp is unique and in the range `[lowest_timestamp, highest_timestamp]`
#[derive(Debug)]
pub struct ShortQueue<T, const N: usize>
where
  T: Cmov + Pod,
{
  // The timestamp of the most recently added element
  highest_timestamp: usize,
  // The timestamp of the oldest element added
  lowest_timestamp: usize,
  // Number of elements in the queue
  pub(crate) size: usize,
  // The array that stores the elements and their timestamps
  pub(crate) elements: ShortArray<ShortQueueElement<T>, N>,
}

impl<T, const N: usize> ShortQueue<T, N>
where
  T: Cmov + Pod + Default,
{
  /// Creates a new empty `ShortQueue` with maximum size `N`.
  pub fn new() -> Self {
    Self { highest_timestamp: 0, lowest_timestamp: 1, size: 0, elements: ShortArray::new() }
  }

  /// Pushes `element` into the queue if `real` is true.
  pub fn maybe_push(&mut self, real: bool, element: T) {
    debug_assert!(!real | (self.size < N));

    self.size.cmov(&(self.size + 1), real);
    self.highest_timestamp.cmov(&(self.highest_timestamp + 1), real);
    let mut inserted = !real;
    let mut lowest_timestamp = self.highest_timestamp;
    for i in 0..self.elements.len() {
      let curr = &mut self.elements.data[i];
      let is_empty = curr.is_empty();
      let should_insert = !inserted & is_empty;
      let is_lowest_timemstamp = !is_empty & (curr.timestamp < lowest_timestamp);
      curr.timestamp.cmov(&self.highest_timestamp, should_insert);
      curr.value.cmov(&element, should_insert);
      lowest_timestamp.cmov(&curr.timestamp, is_lowest_timemstamp);
      inserted |= should_insert;
    }

    debug_assert!(inserted);

    self.lowest_timestamp.cmov(&lowest_timestamp, real);
  }

  /// Pops an element from the queue into `out` if `real` is true.
  pub fn maybe_pop(&mut self, real: bool, out: &mut T) {
    debug_assert!(!real | (self.size > 0));

    self.size.cmov(&(self.size.wrapping_sub(1)), real);
    let mut second_lowest_timestamp = self.highest_timestamp;
    for i in 0..self.elements.len() {
      let curr = &mut self.elements.data[i];
      let is_lowest = curr.timestamp == self.lowest_timestamp;
      let could_be_second_lowest =
        !curr.is_empty() & !is_lowest & (curr.timestamp < second_lowest_timestamp);
      let should_pop = real & is_lowest;
      second_lowest_timestamp.cmov(&curr.timestamp, could_be_second_lowest);
      out.cmov(&curr.value, should_pop);
      curr.timestamp.cmov(&0, should_pop);
    }
    self.lowest_timestamp.cmov(&second_lowest_timestamp, real);
  }
}

impl<T, const N: usize> Length for ShortQueue<T, N>
where
  T: Cmov + Pod + Default,
{
  fn len(&self) -> usize {
    self.size
  }
}

impl<T, const N: usize> Default for ShortQueue<T, N>
where
  T: Cmov + Pod + Default,
{
  fn default() -> Self {
    Self::new()
  }
}

// UNDONE(git-36): Implement ShortStack and LongStack using CircuitORAM
// UNDONE(git-37): Implement LongQueue using CircuitORAM

// UNDONE(git-39): Benchmark ShortQueue

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_short_queue() {
    let mut queue: ShortQueue<u32, 3> = ShortQueue::new();
    assert_eq!(queue.len(), 0);
    queue.maybe_push(true, 1); // ==> [1]
    assert_eq!(queue.len(), 1);

    queue.maybe_push(true, 2); // ==> [1, 2]
    assert_eq!(queue.len(), 2);

    queue.maybe_push(false, 42);
    assert_eq!(queue.len(), 2);

    queue.maybe_push(true, 3); // ==> [1, 2, 3]
    assert_eq!(queue.len(), 3);

    queue.maybe_push(false, 4);
    assert_eq!(queue.len(), 3);

    let mut out = 0;
    queue.maybe_pop(true, &mut out); // ==> [2, 3]
    assert_eq!(out, 1);
    assert_eq!(queue.len(), 2);

    queue.maybe_pop(true, &mut out); // ==> [3]
    assert_eq!(out, 2);
    assert_eq!(queue.len(), 1);

    queue.maybe_pop(false, &mut out);
    assert_eq!(queue.len(), 1);

    queue.maybe_pop(true, &mut out); // ==> []
    assert_eq!(out, 3);
    assert_eq!(queue.len(), 0);

    queue.maybe_pop(false, &mut out);
    assert_eq!(queue.len(), 0);
  }
}
