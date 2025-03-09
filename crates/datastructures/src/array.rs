//! Implements a fixed-size array with a fixed-size element type.
//! The array is oblivious to the access pattern.
//!

use bytemuck::Pod;
use rand::{rngs::ThreadRng, Rng};
use rods_oram::{
  circuit_oram::CircuitORAM,
  linear_oram::{oblivious_read_index, oblivious_write_index},
  recursive_oram::RecursivePositionMap,
};
use rods_primitives::{indexable::Length, traits::Cmov};

/// A fixed sized array defined at compile time.
/// The size of the array is public.
pub type Array<T, const N: usize> = FixedArray<T, N>;
/// A fixed sized array defined at runtime.
/// The size of the array is public.
pub type DArray<T> = DynamicArray<T>;

/// A fixed-size oblivious array, optimal for small sizes.
/// The size of the array is public.
#[repr(C)]
#[derive(Debug)]
pub struct ShortArray<T, const N: usize>
// where T: Cmov Default,
{
  /// The underlying data storage, which is public
  pub(crate) data: [T; N],
}

impl<T, const N: usize> ShortArray<T, N>
where
  T: Cmov + Pod + Default,
{
  /// Creates a new `ShortArray` with the given size `n`.
  pub fn new() -> Self {
    Self { data: [T::default(); N] }
  }

  /// Reads fromm the index
  pub fn read(&self, index: usize, out: &mut T) {
    oblivious_read_index(&self.data, index, out);
  }

  /// Writes to the index
  pub fn write(&mut self, index: usize, value: T) {
    oblivious_write_index(&mut self.data, index, value);
  }
}

impl<T, const N: usize> Length for ShortArray<T, N> {
  fn len(&self) -> usize {
    N
  }
}

impl<T, const N: usize> Default for ShortArray<T, N>
where
  T: Cmov + Pod + Default,
{
  fn default() -> Self {
    Self::new()
  }
}

/// A fixed-size oblivious array, optimal for large sizes.
/// The size of the array is public.
#[repr(C)]
#[derive(Debug)]
pub struct LongArray<T, const N: usize>
where
  T: Cmov + Pod,
{
  /// The actual data storage oram
  data: CircuitORAM<T>,
  /// The position map for the oram
  pos_map: RecursivePositionMap,
  /// The local rng for the oram
  rng: ThreadRng,
}
impl<T, const N: usize> LongArray<T, N>
where
  T: Cmov + Pod + Default + std::fmt::Debug,
{
  /// Creates a new `LongArray` with the given size `n`.
  pub fn new() -> Self {
    Self { data: CircuitORAM::new(N), pos_map: RecursivePositionMap::new(N), rng: rand::rng() }
  }

  /// Reads fromm the index
  pub fn read(&mut self, index: usize, out: &mut T) {
    let new_pos = self.rng.random_range(0..N);
    let old_pos = self.pos_map.access_position(index, new_pos);
    self.data.read(old_pos, new_pos, index, out);
  }

  /// Writes to the index
  pub fn write(&mut self, index: usize, value: T) {
    let new_pos = self.rng.random_range(0..N);
    let old_pos = self.pos_map.access_position(index, new_pos);
    self.data.write(old_pos, new_pos, index, value);
  }
}

impl<T: Cmov + Pod, const N: usize> Length for LongArray<T, N> {
  fn len(&self) -> usize {
    N
  }
}

impl<T: Cmov + Pod + Default + std::fmt::Debug, const N: usize> Default for LongArray<T, N> {
  fn default() -> Self {
    Self::new()
  }
}

// UNDONE(git-52): Optimize SHORT_ARRAY_THRESHOLD
const SHORT_ARRAY_THRESHOLD: usize = 128;

/// A fixed-size array that switches between `ShortArray` and `LongArray` based on the size.
/// The size of the array is public.
///
/// # Invariants
/// if `N <= SHORT_ARRAY_THRESHOLD`, then `ShortArray` is used, otherwise `LongArray` is used.
#[repr(C)]
#[derive(Debug)]
pub enum FixedArray<T, const N: usize>
where
  T: Cmov + Pod,
{
  /// Short variant, linear scan
  Short(ShortArray<T, N>),
  /// Long variant, oram
  Long(LongArray<T, N>),
}

impl<T, const N: usize> FixedArray<T, N>
where
  T: Cmov + Pod + Default + std::fmt::Debug,
{
  /// Creates a new `LongArray` with the given size `n`.
  pub fn new() -> Self {
    if N <= SHORT_ARRAY_THRESHOLD {
      Self::Short(ShortArray::new())
    } else {
      Self::Long(LongArray::new())
    }
  }

  /// Reads fromm the index
  pub fn read(&mut self, index: usize, out: &mut T) {
    if N <= SHORT_ARRAY_THRESHOLD {
      // Do an unsafe cast to avoid borrowing issues
      let short_array: &mut ShortArray<T, N>;
      unsafe {
        short_array = std::mem::transmute::<&mut Self, &mut ShortArray<T, N>>(self);
      }
      short_array.read(index, out);
    } else {
      let long_array: &mut LongArray<T, N>;
      unsafe {
        long_array = std::mem::transmute::<&mut Self, &mut LongArray<T, N>>(self);
      }
      long_array.read(index, out);
    }
  }

  /// Writes to the index
  pub fn write(&mut self, index: usize, value: T) {
    if N <= SHORT_ARRAY_THRESHOLD {
      // Do an unsafe cast to avoid borrowing issues
      let short_array: &mut ShortArray<T, N>;
      unsafe {
        short_array = std::mem::transmute::<&mut Self, &mut ShortArray<T, N>>(self);
      }
      short_array.write(index, value);
    } else {
      let long_array: &mut LongArray<T, N>;
      unsafe {
        long_array = std::mem::transmute::<&mut Self, &mut LongArray<T, N>>(self);
      }
      long_array.write(index, value);
    }
  }
}

impl<T: Cmov + Pod, const N: usize> Length for FixedArray<T, N> {
  fn len(&self) -> usize {
    N
  }
}

impl<T: Cmov + Pod + Default + std::fmt::Debug, const N: usize> Default for FixedArray<T, N> {
  fn default() -> Self {
    Self::new()
  }
}

/// An array whose size is determined at runtime.
/// The size of the array is public.
/// The array is oblivious to the access pattern.
///
#[derive(Debug)]
pub struct DynamicArray<T>
where
  T: Cmov + Pod,
{
  /// The actual data storage oram
  data: CircuitORAM<T>,
  /// The position map for the oram
  pos_map: RecursivePositionMap,
  /// The local rng for the oram
  rng: ThreadRng,
}

impl<T> DynamicArray<T>
where
  T: Cmov + Pod + Default + std::fmt::Debug,
{
  /// Creates a new `LongArray` with the given size `n`.
  pub fn new(n: usize) -> Self {
    Self { data: CircuitORAM::new(n), pos_map: RecursivePositionMap::new(n), rng: rand::rng() }
  }

  /// Resizes the array to have `n` elements.
  pub fn resize(&mut self, n: usize) {
    let mut new_array = Self::new(n);
    for i in 0..n {
      let mut value = Default::default();
      self.read(i, &mut value);
      new_array.write(i, value);
    }
    // UNDONE(git-57): Is this 0 cost in rust? DynamicArray is noncopy, so I would expect move semantics here, but double check
    *self = new_array;
  }

  /// Reads fromm the index
  pub fn read(&mut self, index: usize, out: &mut T) {
    let new_pos = self.rng.random_range(0..self.data.max_n);
    let old_pos = self.pos_map.access_position(index, new_pos);
    self.data.read(old_pos, new_pos, index, out);
  }

  /// Writes to the index
  pub fn write(&mut self, index: usize, value: T) {
    let new_pos = self.rng.random_range(0..self.data.max_n);
    let old_pos = self.pos_map.access_position(index, new_pos);
    self.data.write(old_pos, new_pos, index, value);
  }

  /// Updates the value at the index using the update function.
  pub fn update<R, F>(&mut self, index: usize, update_func: F) -> (bool, R)
  where
    F: FnOnce(&mut T) -> R,
  {
    let new_pos = self.rng.random_range(0..self.data.max_n);
    let old_pos = self.pos_map.access_position(index, new_pos);
    self.data.update(old_pos, new_pos, index, update_func)
  }
}

impl<T: Cmov + Pod> Length for DynamicArray<T> {
  fn len(&self) -> usize {
    self.data.max_n
  }
}

// UNDONE(git-29): Test short array
// UNDONE(git-29): Test long array
// UNDONE(git-29): Test fixed array
// UNDONE(git-29): Test dynamic array
// UNDONE(git-30): Benchmark short array
// UNDONE(git-30): Benchmark long array
// UNDONE(git-30): Benchmark fixed array
// UNDONE(git-30): Benchmark dynamic array
// If in rust update monorfization is trully 0-cost, ten we can implement the following two via an update function:
// UNDONE(git-31): Implement versions of read and write that hide the operation from the caller.
// UNDONE(git-31): Implement read and write that have an enable flag (maybe_read, maybe_write).
