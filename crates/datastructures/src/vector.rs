//! Implements variable lenght vectors.
//! The vector is oblivious to the access pattern.

use std::fmt::Debug;

use crate::array::DynamicArray;
use bytemuck::Pod;
use rods_primitives::{indexable::Length, traits::Cmov};

/// Implements a variable lenght vector.
/// Leaks the length rounded to the next power of two.
/// The vector is oblivious to the access pattern.
pub type Vector<T> = EagerVector<T>;

/// Implements a variable lenght vector.
/// Leaks the length rounded to the next power of two.
/// The vector is oblivious to the access pattern.
#[derive(Debug)]
pub struct EagerVector<T>
where
  T: Cmov + Pod,
{
  /// The length of the vector (number of elements in the vector)
  n: usize,
  /// The underlying data storage
  data: DynamicArray<T>,
}

impl<T> EagerVector<T>
where
  T: Cmov + Pod + Default + Debug,
{
  /// Creates a new `EagerVector` with the given size `n`.
  pub fn new() -> Self {
    Self { n: 0, data: DynamicArray::new(1) }
  }

  /// Reads from the index
  pub fn read(&mut self, index: usize, out: &mut T) {
    assert!(index < self.n);
    self.data.read(index, out);
  }

  /// Writes to the index
  pub fn write(&mut self, index: usize, value: T) {
    assert!(index < self.n);
    self.data.write(index, value);
  }

  /// Pushes a new element to the end of the vector
  pub fn push_back(&mut self, value: T) {
    if self.n == self.data.len() {
      self.data.resize(2 * self.n);
    }
    self.data.write(self.n, value);
    self.n += 1;
  }

  /// Pops the last element from the vector, returning it
  pub fn pop_back(&mut self) -> T {
    assert!(self.n > 0);
    self.n -= 1;
    let mut value = Default::default();
    self.data.read(self.n, &mut value);
    value
  }

  /// Returns the current capacity of the vector: `len() <= capacity()`
  pub fn capacity(&self) -> usize {
    self.data.len()
  }
}

impl<T: Cmov + Pod> Length for EagerVector<T> {
  fn len(&self) -> usize {
    self.n
  }
}

impl<T: Cmov + Pod + Default + Debug> Default for EagerVector<T> {
  fn default() -> Self {
    Self::new()
  }
}

// UNDONE(git-40): Should we implement LazyVector? (i.e. it grows lazily when needed, without leaking the length increase at powers of 2 directly)

// UNDONE(git-41): Test EagerVector
// UNDONE(git-42): Benchmark EagerVector
