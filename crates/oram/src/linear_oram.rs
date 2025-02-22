//! Linear Scan ORAM
//!
use bytemuck::Pod;
use rods_primitives::traits::Cmov;

/// A simple indexable ORAM that does a linear scan for each access
#[derive(Debug)]
pub struct LinearOram<T>
where
  T: Cmov + Pod,
{
  data: Vec<T>,
}

impl<T> LinearOram<T>
where
  T: Cmov + Pod + Default,
{
  ///initialization
  pub fn new(max_n: usize) -> Self {
    Self { data: vec![T::default(); max_n] }
  }
  ///linear scan the entire array, move the element out when index matches
  pub fn read(&self, index: usize, ret: &mut T) {
    for i in 0..self.data.len() {
      ret.cmov(&self.data[i], i == index);
    }
  }
  ///linear scan the entire array, write to the index if the index matches
  pub fn write(&mut self, index: usize, value: T) {
    for i in 0..self.data.len() {
      self.data[i].cmov(&value, i == index);
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_read() {
    let default = 0;
    let index = 3;
    let oram = LinearOram::<u32>::new(10);
    let mut ret = 0;
    oram.read(index, &mut ret);
    assert_eq!(ret, default);
  }

  #[test]
  fn test_write() {
    let default = 0;
    let new_value = 25;
    let index = 3;
    let mut oram = LinearOram::<u32>::new(10);
    oram.write(index, new_value);
    let mut ret = default;
    oram.read(index, &mut ret);
    assert_eq!(ret, new_value);
  }
}
