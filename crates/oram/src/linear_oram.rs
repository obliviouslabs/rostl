//! Linear Scan ORAM
//!
use crate::prelude::K;
use bytemuck::Pod;
use rods_primitives::traits::Cmov;

/// A simple indexable ORAM that does a linear scan for each access
#[derive(Debug)]
pub struct LinearORAM<T>
where
  T: Cmov + Pod,
{
  /// The actual data storage array
  pub data: Vec<T>,
}

#[inline]
/// Performs an oblivious read and update at the specified index.
///
/// # Arguments
///
/// * `data` - A mutable slice of data.
/// * `index` - The index to read and update.
/// * `ret` - A mutable reference to store the read value.
/// * `value` - The value to write at the specified index.
pub fn oblivious_read_update_index<T: Cmov>(data: &mut [T], index: usize, ret: &mut T, value: T) {
  debug_assert!(index < data.len());
  for (i, item) in data.iter_mut().enumerate() {
    let choice = i == index;
    ret.cmov(item, choice);
    item.cmov(&value, choice);
  }
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
}
