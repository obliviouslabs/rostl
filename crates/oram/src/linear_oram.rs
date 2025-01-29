//! Linear Scan ORAM
//!
use rods_primitives::{indexable::Indexable, traits::Cmov};
use std::marker::PhantomData;

/// A simple indexable ORAM that does a linear scan for each access
#[derive(Debug)]
pub struct LinearOram<V, T, const SIZE: usize>
where
  T: Cmov + Copy + Default,
  V: Indexable<T>,
{
  data: V,
  _marker: PhantomData<T>,
}

impl<V, T, const SIZE: usize> LinearOram<V, T, SIZE>
where
  T: Cmov + Copy + Default,
  V: Indexable<T>,
{
  ///initialization
  pub const fn new(d: V) -> Self {
    Self { data: d, _marker: PhantomData }
  }
  ///linear scan the entyre array, move the element out when index matches
  pub fn read(&self, index: usize) -> T {
    let mut ret = T::default();
    for i in 0..self.data.len() {
      ret.cmov(&self.data[i], i == index);
    }
    ret
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_read() {
    let vec = vec![25; 10];
    let oram = LinearOram::<Vec<u32>, u32, 128>::new(vec);
    assert_eq!(oram.read(3), 25);
  }
}
