//! Linear Scan ORAM
//!
use rods_primitives::{indexable::Indexable, traits::Cmov};
use std::marker::PhantomData;

/// A simple indexable ORAM that does a linear scan for each access
#[derive(Debug)]
pub struct LinearOram<V, T>
where
  T: Cmov,
  V: Indexable<T>,
{
  data: V,
  _marker: PhantomData<T>,
}

impl<V, T> LinearOram<V, T>
where
  T: Cmov,
  V: Indexable<T>,
{
  ///initialization
  pub const fn new(d: V) -> Self {
    Self { data: d, _marker: PhantomData }
  }
  ///linear scan the entire array, move the element out when index matches
  pub fn read(&self, index: usize, ret: &mut T){
    for i in 0..self.data.len() {
      ret.cmov(&self.data[i], i == index);
    }
  }
  ///linear scan the entire array, write to the index if the index matches
  pub fn write(&mut self, index: usize, value: T){
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
    let default = 25;
    let index = 3;
    let vec = vec![default; 10];
    let oram = LinearOram::<Vec<u32>, u32>::new(vec);
    let mut ret = 0;
    oram.read(index,&mut ret);
    assert_eq!(ret, default);
  }

  #[test]
  fn test_write() {
    let default = 25;
    let new_value = 0;
    let index = 3;
    let vec = vec![default; 10];
    let mut oram = LinearOram::<Vec<u32>, u32>::new(vec);
    oram.write(index, new_value);
    let mut ret = 0;
    oram.read(index,&mut ret);
    assert_eq!(ret, new_value);
  }
}
