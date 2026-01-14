//! Implements a fixed-size array with a fixed-size element type.
//! The array is oblivious to the access pattern.
//!

use std::{array::from_fn, mem::ManuallyDrop};

use bytemuck::Pod;
use rand::{rng, Rng};
use rostl_oram::{
  circuit_oram::CircuitORAM,
  linear_oram::{oblivious_read_index, oblivious_write_index},
  prelude::PositionType,
  recursive_oram::RecursivePositionMap,
};
use rostl_primitives::{indexable::Length, traits::Cmov};

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

  /// Reads from the index
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
}
impl<T, const N: usize> LongArray<T, N>
where
  T: Cmov + Pod + Default + std::fmt::Debug,
{
  /// Creates a new `LongArray` with the given size `n`.
  pub fn new() -> Self {
    Self { data: CircuitORAM::new(N), pos_map: RecursivePositionMap::new(N) }
  }

  /// Reads from the index
  pub fn read(&mut self, index: usize, out: &mut T) {
    let new_pos = rng().random_range(0..N as PositionType);
    let old_pos = self.pos_map.access_position(index, new_pos);
    self.data.read(old_pos, new_pos, index, out);
  }

  /// Writes to the index
  pub fn write(&mut self, index: usize, value: T) {
    let new_pos = rng().random_range(0..N as PositionType);
    let old_pos = self.pos_map.access_position(index, new_pos);
    self.data.write_or_insert(old_pos, new_pos, index, value);
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
///
#[repr(C)]
pub union FixedArray<T, const N: usize>
where
  T: Cmov + Pod,
{
  /// Short variant, linear scan
  short: ManuallyDrop<ShortArray<T, N>>,
  /// Long variant, oram
  long: ManuallyDrop<LongArray<T, N>>,
}

impl<T, const N: usize> Drop for FixedArray<T, N>
where
  T: Cmov + Pod,
{
  fn drop(&mut self) {
    if N <= SHORT_ARRAY_THRESHOLD {
      unsafe {
        ManuallyDrop::drop(&mut self.short);
      }
    } else {
      unsafe {
        ManuallyDrop::drop(&mut self.long);
      }
    }
  }
}

impl<T, const N: usize> std::fmt::Debug for FixedArray<T, N>
where
  T: Cmov + Pod + std::fmt::Debug,
{
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    if N <= SHORT_ARRAY_THRESHOLD {
      let short_array: &ManuallyDrop<ShortArray<T, N>>;
      unsafe {
        short_array = &self.short;
      }
      short_array.fmt(f)
    } else {
      let long_array: &ManuallyDrop<LongArray<T, N>>;
      unsafe {
        long_array = &self.long;
      }
      long_array.fmt(f)
    }
  }
}

impl<T, const N: usize> FixedArray<T, N>
where
  T: Cmov + Pod + Default + std::fmt::Debug,
{
  /// Creates a new `LongArray` with the given size `n`.
  pub fn new() -> Self {
    if N <= SHORT_ARRAY_THRESHOLD {
      FixedArray { short: ManuallyDrop::new(ShortArray::new()) }
    } else {
      FixedArray { long: ManuallyDrop::new(LongArray::new()) }
    }
  }

  /// Reads from the index
  pub fn read(&mut self, index: usize, out: &mut T) {
    if N <= SHORT_ARRAY_THRESHOLD {
      // Do an unsafe cast to avoid borrowing issues
      let short_array: &mut ManuallyDrop<ShortArray<T, N>>;
      unsafe {
        short_array = &mut self.short;
      }
      short_array.read(index, out);
    } else {
      let long_array: &mut ManuallyDrop<LongArray<T, N>>;
      unsafe {
        long_array = &mut self.long;
      }
      long_array.read(index, out);
    }
  }

  /// Writes to the index
  pub fn write(&mut self, index: usize, value: T) {
    if N <= SHORT_ARRAY_THRESHOLD {
      // Do an unsafe cast to avoid borrowing issues
      let short_array: &mut ManuallyDrop<ShortArray<T, N>>;
      unsafe {
        short_array = &mut self.short;
      }
      short_array.write(index, value);
    } else {
      let long_array: &mut ManuallyDrop<LongArray<T, N>>;
      unsafe {
        long_array = &mut self.long;
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

// impl<T: Cmov + Pod + Default + std::fmt::Debug, const N: usize> Drop for FixedArray<T, N> {
//   fn drop(&mut self) {
//     if N <= SHORT_ARRAY_THRESHOLD {
//       let short_array: &mut ShortArray<T, N>;
//       unsafe {
//         short_array = std::mem::transmute::<&mut Self, &mut ShortArray<T, N>>(self);
//       }
//       std::mem::drop(short_array);
//     } else {
//       let long_array: &mut LongArray<T, N>;
//       unsafe {
//         long_array = std::mem::transmute::<&mut Self, &mut LongArray<T, N>>(self);
//       }
//       std::mem::drop(long_array);
//     }
//   }
// }

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
}

impl<T> DynamicArray<T>
where
  T: Cmov + Pod + Default + std::fmt::Debug,
{
  /// Creates a new `LongArray` with the given size `n`.
  pub fn new(n: usize) -> Self {
    Self { data: CircuitORAM::new(n), pos_map: RecursivePositionMap::new(n) }
  }

  /// Resizes the array to have `n` elements.
  pub fn resize(&mut self, n: usize) {
    let mut new_array = Self::new(n);
    for i in 0..self.len() {
      let mut value = Default::default();
      self.read(i, &mut value);
      new_array.write(i, value);
    }
    // UNDONE(git-57): Is this 0 cost in rust? DynamicArray is noncopy, so I would expect move semantics here, but double check
    *self = new_array;
  }

  /// Reads from the index
  pub fn read(&mut self, index: usize, out: &mut T) {
    let new_pos = rng().random_range(0..self.len() as PositionType);
    let old_pos = self.pos_map.access_position(index, new_pos);
    self.data.read(old_pos, new_pos, index, out);
  }

  /// Writes to the index
  pub fn write(&mut self, index: usize, value: T) {
    let new_pos = rng().random_range(0..self.len() as PositionType);
    let old_pos = self.pos_map.access_position(index, new_pos);
    self.data.write_or_insert(old_pos, new_pos, index, value);
  }

  /// Updates the value at the index using the update function.
  pub fn update<R, F>(&mut self, index: usize, update_func: F) -> (bool, R)
  where
    F: FnOnce(&mut T) -> R,
  {
    let new_pos = rng().random_range(0..self.len() as PositionType);
    let old_pos = self.pos_map.access_position(index, new_pos);
    self.data.update(old_pos, new_pos, index, update_func)
  }
}

impl<T: Cmov + Pod> Length for DynamicArray<T> {
  #[inline(always)]
  fn len(&self) -> usize {
    self.pos_map.n
  }
}

/// A set of `W` subarrays that can be used to store a fixed number of total elements defined at `new` time. It is leaked which subarray is being accessed.
///
#[derive(Debug)]
pub struct MultiWayArray<T, const W: usize>
where
  T: Cmov + Pod,
{
  /// The actual data storage oram
  data: CircuitORAM<T>,
  /// The position maps for each subarray
  pos_map: [RecursivePositionMap; W],
}

impl<T, const W: usize> MultiWayArray<T, W>
where
  T: Cmov + Pod + Default + std::fmt::Debug,
{
  /// Creates a new `MultiWayArray` with the given size `n`.
  pub fn new(n: usize) -> Self {
    assert!(W.is_power_of_two(), "W must be a power of two due to all the ilog2's here");
    Self { data: CircuitORAM::new(n), pos_map: from_fn(|_| RecursivePositionMap::new(n)) }
  }

  fn get_real_index(&self, subarray: usize, index: usize) -> usize {
    debug_assert!(subarray < W, "Subarray index out of bounds");
    debug_assert!(index < self.len(), "Index out of bounds");
    (index << W.ilog2()) | subarray
  }

  /// Reads from the subarray and index
  pub fn read(&mut self, subarray: usize, index: usize, out: &mut T) {
    let new_pos = rng().random_range(0..self.len() as PositionType);
    let old_pos = self.pos_map[subarray].access_position(index, new_pos);
    let real_index = self.get_real_index(subarray, index);
    self.data.read(old_pos, new_pos, real_index, out);
  }

  /// Writes to the subarray and index
  pub fn write(&mut self, subarray: usize, index: usize, value: T) {
    let new_pos = rng().random_range(0..self.len() as PositionType);
    let old_pos = self.pos_map[subarray].access_position(index, new_pos);
    let real_index = self.get_real_index(subarray, index);
    self.data.write_or_insert(old_pos, new_pos, real_index, value);
  }

  /// Updates the value at the subarray and index using the update function.
  pub fn update<R, F>(&mut self, subarray: usize, index: usize, update_func: F) -> (bool, R)
  where
    F: FnOnce(&mut T) -> R,
  {
    let new_pos = rng().random_range(0..self.len() as PositionType);
    let old_pos = self.pos_map[subarray].access_position(index, new_pos);
    let real_index = self.get_real_index(subarray, index);
    self.data.update(old_pos, new_pos, real_index, update_func)
  }
}

impl<T: Cmov + Pod, const W: usize> Length for MultiWayArray<T, W> {
  #[inline(always)]
  fn len(&self) -> usize {
    self.pos_map[0].n
  }
}

// UNDONE(git-30): Benchmark short array
// UNDONE(git-30): Benchmark long array
// UNDONE(git-30): Benchmark fixed array
// UNDONE(git-30): Benchmark dynamic array
// If in rust update monorfization is truly 0-cost, ten we can implement the following two via an update function:
// UNDONE(git-31): Implement versions of read and write that hide the operation from the caller.
// UNDONE(git-31): Implement read and write that have an enable flag (maybe_read, maybe_write).

#[cfg(test)]
#[allow(clippy::reversed_empty_ranges)]
mod tests {
  use super::*;

  macro_rules! m_test_fixed_array_exhaustive {
    ($arraytp:ident, $valtp:ty, $size:expr) => {{
      println!("Testing {} with size {}", stringify!($arraytp), $size);
      let mut arr = $arraytp::<$valtp, $size>::new();
      assert_eq!(arr.len(), $size);
      for i in 0..$size {
        let mut value = Default::default();
        arr.read(i, &mut value);
        assert_eq!(value, Default::default());
      }
      assert_eq!(arr.len(), $size);
      for i in 0..$size {
        let value = i as $valtp;
        arr.write(i, value);
      }
      assert_eq!(arr.len(), $size);
      for i in 0..$size {
        let mut value = Default::default();
        arr.read(i, &mut value);
        let v = i as $valtp;
        assert_eq!(value, v);
      }
      assert_eq!(arr.len(), $size);
    }};
  }

  macro_rules! m_test_multiway_array_exhaustive {
    ($arraytp:ident, $valtp:ty, $size:expr, $ways:expr) => {{
      println!("Testing {} with size {}", stringify!($arraytp), $size);
      let mut arr = $arraytp::<$valtp, $ways>::new($size);
      assert_eq!(arr.len(), $size);
      for w in 0..$ways {
        for i in 0..$size {
          let mut value = Default::default();
          arr.read(w, i, &mut value);
          assert_eq!(value, Default::default());
        }
      }
      assert_eq!(arr.len(), $size);

      for w in 0..$ways {
        for i in 0..($size / $ways) {
          let value = (i + w) as $valtp;
          arr.write(w, i, value);
        }
      }
      assert_eq!(arr.len(), $size);
      for w in 0..$ways {
        for i in 0..($size / $ways) {
          let mut value = Default::default();
          arr.read(w, i, &mut value);
          let v = (i + w) as $valtp;
          assert_eq!(value, v);
        }
      }
      assert_eq!(arr.len(), $size);
    }};
  }

  macro_rules! m_test_dynamic_array_exhaustive {
    ($arraytp:ident, $valtp:ty, $size:expr) => {{
      println!("Testing {} with size {}", stringify!($arraytp), $size);
      let mut arr = $arraytp::<$valtp>::new($size);
      assert_eq!(arr.len(), $size);
      for i in 0..$size {
        let mut value = Default::default();
        arr.read(i, &mut value);
        assert_eq!(value, Default::default());
      }
      assert_eq!(arr.len(), $size);
      for i in 0..$size {
        let value = i as $valtp;
        arr.write(i, value);
      }
      assert_eq!(arr.len(), $size);
      for i in 0..$size {
        let mut value = Default::default();
        arr.read(i, &mut value);
        let v = i as $valtp;
        assert_eq!(value, v);
      }
      assert_eq!(arr.len(), $size);
      arr.resize($size + 1);
      assert_eq!(arr.len(), $size + 1);
      for i in 0..$size {
        let mut value = Default::default();
        arr.read(i, &mut value);
        let v = i as $valtp;
        assert_eq!(value, v);
      }
      assert_eq!(arr.len(), $size + 1);
      for i in $size..($size + 1) {
        let mut value = Default::default();
        arr.read(i, &mut value);
        assert_eq!(value, Default::default());
      }
      assert_eq!(arr.len(), $size + 1);
      arr.resize(2 * $size);
      assert_eq!(arr.len(), 2 * $size);
      for i in 0..$size {
        let mut value = Default::default();
        arr.read(i, &mut value);
        let v = i as $valtp;
        assert_eq!(value, v);
      }
      assert_eq!(arr.len(), 2 * $size);
      for i in $size..(2 * $size) {
        let mut value = Default::default();
        arr.read(i, &mut value);
        assert_eq!(value, Default::default());
      }
      assert_eq!(arr.len(), 2 * $size);
      // UNDONE(git-29): Test update
    }};
  }

  #[test]
  fn test_fixed_arrays() {
    m_test_fixed_array_exhaustive!(ShortArray, u32, 1);
    m_test_fixed_array_exhaustive!(ShortArray, u32, 2);
    m_test_fixed_array_exhaustive!(ShortArray, u32, 3);
    m_test_fixed_array_exhaustive!(ShortArray, u64, 15);
    m_test_fixed_array_exhaustive!(ShortArray, u8, 33);
    m_test_fixed_array_exhaustive!(ShortArray, u64, 200);

    // m_test_fixed_array_exhaustive!(LongArray, u32, 1);
    m_test_fixed_array_exhaustive!(LongArray, u32, 2);
    m_test_fixed_array_exhaustive!(LongArray, u32, 3);
    m_test_fixed_array_exhaustive!(LongArray, u64, 15);
    m_test_fixed_array_exhaustive!(LongArray, u8, 33);

    m_test_fixed_array_exhaustive!(FixedArray, u32, 1);
    m_test_fixed_array_exhaustive!(FixedArray, u32, 2);
    m_test_fixed_array_exhaustive!(FixedArray, u32, 3);
    m_test_fixed_array_exhaustive!(FixedArray, u64, 15);
    m_test_fixed_array_exhaustive!(FixedArray, u8, 33);
    m_test_fixed_array_exhaustive!(FixedArray, u64, 200);
  }

  #[test]
  fn test_multiway_array() {
    // m_test_multiway_array_exhaustive!(MultiWayArray, u32, 1, 1);
    m_test_multiway_array_exhaustive!(MultiWayArray, u32, 2, 1);
    m_test_multiway_array_exhaustive!(MultiWayArray, u32, 3, 1);
    m_test_multiway_array_exhaustive!(MultiWayArray, u64, 15, 1);
    m_test_multiway_array_exhaustive!(MultiWayArray, u8, 33, 1);
    m_test_multiway_array_exhaustive!(MultiWayArray, u64, 200, 1);

    // m_test_multiway_array_exhaustive!(MultiWayArray, u32, 1, 2);
    m_test_multiway_array_exhaustive!(MultiWayArray, u32, 2, 2);
    m_test_multiway_array_exhaustive!(MultiWayArray, u32, 3, 2);
    m_test_multiway_array_exhaustive!(MultiWayArray, u64, 15, 2);
    m_test_multiway_array_exhaustive!(MultiWayArray, u8, 33, 2);
    m_test_multiway_array_exhaustive!(MultiWayArray, u64, 200, 2);

    // m_test_multiway_array_exhaustive!(MultiWayArray, u32, 1, 4);
    m_test_multiway_array_exhaustive!(MultiWayArray, u32, 2, 4);
    m_test_multiway_array_exhaustive!(MultiWayArray, u32, 3, 4);
    m_test_multiway_array_exhaustive!(MultiWayArray, u64, 15, 4);
    m_test_multiway_array_exhaustive!(MultiWayArray, u8, 33, 4);
    m_test_multiway_array_exhaustive!(MultiWayArray, u64, 200, 4);
  }

  #[test]
  fn test_dynamic_array() {
    // m_test_dynamic_array_exhaustive!(DynamicArray, u32, 1);
    m_test_dynamic_array_exhaustive!(DynamicArray, u32, 2);
    m_test_dynamic_array_exhaustive!(DynamicArray, u32, 3);
    m_test_dynamic_array_exhaustive!(DynamicArray, u64, 15);
    m_test_dynamic_array_exhaustive!(DynamicArray, u8, 33);
    m_test_dynamic_array_exhaustive!(DynamicArray, u64, 200);
  }
}
