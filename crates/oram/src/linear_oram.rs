//! Linear Scan ORAM
//!
use std::borrow::Borrow;

use crate::prelude::K;
use bytemuck::Pod;
use rostl_primitives::traits::Cmov;
use rostl_sort::rotate::rotate_left;

/// A simple indexable ORAM that does a linear scan for each access
#[derive(Debug)]
pub struct LinearORAM<T>
where
  T: Cmov + Pod,
{
  /// The actual data storage array
  pub data: Vec<T>,
}

/// Performs an oblivious read and update at the specified index.
///
/// # Arguments
///
/// * `data` - A mutable slice of data.
/// * `index` - The index to read and update, if out of bounds, the function will not modify `ret` or `data`.
/// * `ret` - A mutable reference to store the read value.
/// * `value` - The value to write at the specified index.
///
/// # Oblivious
/// * Memory access pattern depends only on `data.len()`
#[inline]
pub fn oblivious_read_update_index<T: Cmov>(data: &mut [T], index: usize, ret: &mut T, value: T) {
  for (i, item) in data.iter_mut().enumerate() {
    let choice = i == index;
    ret.cmov(item, choice);
    item.cmov(&value, choice);
  }
}

/// Performs an oblivious read at the specified index.
///
/// # Arguments
///
/// * `data` - A slice of data.
/// * `index` - The index to read. If out of bounds, the function will not modify `out`.
/// * `out` - A mutable reference to store the read value.
///
/// # Oblivious
/// * Memory access pattern depends only on `data.len()`
#[inline]
pub fn oblivious_read_index<T: Cmov>(data: &[T], index: usize, out: &mut T) {
  for (i, item) in data.iter().enumerate() {
    let choice = i == index;
    out.cmov(item, choice);
  }
}

/// Performs an oblivious write at the specified index.
///
/// # Arguments
///
/// * `data` - A mutable slice of data.
/// * `index` - The index to write, if out of bounds, the function will not modify `data`.
/// * `value` - The value to write at the specified index.
///
/// # Oblivious
/// * Memory access pattern depends only on `data.len()`
#[inline]
pub fn oblivious_write_index<T: Cmov, U: Borrow<T>>(data: &mut [T], index: usize, value: U) {
  for (i, item) in data.iter_mut().enumerate() {
    let choice = i == index;
    item.cmov(value.borrow(), choice);
  }
}

/// Copies a range from `src` to `dst` starting at `src_offset` respectively, if there are out of bound bytes, the values after the overflow offset in `dst` will have arbitrary data from other parts of `old(dst)`. If `src_offset >= src.len()`, `dst` will remain unchanged.
///
/// # Arguments
/// * `dst` - A mutable slice of destination data.
/// * `src` - A slice of source data.
/// * `src_offset` - The starting offset in the source slice.
///
/// # Oblivious
/// * Memory access pattern depends only on `dst.len()` and `src.len()`.
///
/// # Complexity
/// * Time: `O(dst.len() + src.len())`
/// * Space: `O(1)`
#[inline]
pub fn oblivious_memcpy<T: Cmov + Copy>(dst: &mut [T], src: &[T], src_offset: usize) {
  let len = dst.len();
  for (i, item) in src.iter().enumerate() {
    let choice = (i >= src_offset) && (i < src_offset + len);
    dst[i % len].cmov(item, choice);
  }
  let mut shift_amount = src_offset % len;
  shift_amount.cmov(&0, src_offset >= src.len());
  rotate_left(dst, shift_amount);
}

impl<T> LinearORAM<T>
where
  T: Cmov + Pod + Default + std::fmt::Debug,
{
  ///initialization
  pub fn new(max_n: usize) -> Self {
    Self { data: vec![T::default(); max_n] }
  }
  ///linear scan the entire array, move the element out when index matches
  pub fn read(&self, index: K, ret: &mut T) {
    debug_assert!(index < self.data.len());
    for i in 0..self.data.len() {
      let choice = i == index;
      ret.cmov(&self.data[i], choice);
    }
  }
  ///linear scan the entire array, write to the index if the index matches
  pub fn write(&mut self, index: K, value: T) {
    for i in 0..self.data.len() {
      let choice = i == index;
      self.data[i].cmov(&value, choice);
    }
  }

  ///linear scan the entire array, read the element out when index matches, write to the index if the index matches
  pub fn read_update(&mut self, index: K, value: T, ret: &mut T) {
    oblivious_read_update_index(&mut self.data, index, ret, value);
  }

  #[cfg(test)]
  pub(crate) fn print_for_debug(&self) {
    for i in 0..self.data.len() {
      print!("{:?}, ", self.data[i]);
    }
    println!();
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_read() {
    let default = 0;
    let index = 3;
    let oram = LinearORAM::<u32>::new(10);
    let mut ret = 0;
    oram.read(index, &mut ret);
    assert_eq!(ret, default);
  }

  #[test]
  fn test_write() {
    let default = 0;
    let new_value = 25;
    let index = 3;
    let mut oram = LinearORAM::<u32>::new(10);
    oram.write(index, new_value);
    let mut ret = default;
    oram.read(index, &mut ret);
    assert_eq!(ret, new_value);
  }

  #[test]
  fn test_oblivious_memcpy() {
    let src = vec![1u8, 2, 3, 4, 5, 6, 7, 8, 9, 10];
    let mut dst = vec![0u8; 5];

    oblivious_memcpy(&mut dst, &src, 0);
    assert_eq!(dst, vec![1, 2, 3, 4, 5]);
    oblivious_memcpy(&mut dst, &src, 1);
    assert_eq!(dst, vec![2, 3, 4, 5, 6]);
    oblivious_memcpy(&mut dst, &src, 2);
    assert_eq!(dst, vec![3, 4, 5, 6, 7]);
    oblivious_memcpy(&mut dst, &src, 3);
    assert_eq!(dst, vec![4, 5, 6, 7, 8]);
    oblivious_memcpy(&mut dst, &src, 4);
    assert_eq!(dst, vec![5, 6, 7, 8, 9]);
    oblivious_memcpy(&mut dst, &src, 5);
    assert_eq!(dst, vec![6, 7, 8, 9, 10]);
    oblivious_memcpy(&mut dst, &src, 6);
    assert_eq!(dst[..4], vec![7, 8, 9, 10]);
    oblivious_memcpy(&mut dst, &src, 7);
    assert_eq!(dst[..3], vec![8, 9, 10]);
    oblivious_memcpy(&mut dst, &src, 8);
    assert_eq!(dst[..2], vec![9, 10]);
    oblivious_memcpy(&mut dst, &src, 9);
    assert_eq!(dst[..1], vec![10]);
    oblivious_memcpy(&mut dst, &src, 10);
  }
}
