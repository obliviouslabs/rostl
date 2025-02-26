//! Basic shuffle algorithm.
use rand::random;
use rods_primitives::indexable::Indexable;
use rods_primitives::traits::Cmov;
use rods_sort::bitonic::bitonic_sort;
use std::cmp::Ordering;

/// Basic shuffle algorithm using bitonic sort.
pub fn basic_shuffle<T, C>(arr: &mut C)
where
  T: Cmov + Copy,
  C: Indexable<T>,
{
  let mut wrapped = wrap_data(arr);
  basic_shuffle_inner(arr, &mut wrapped);
}

#[derive(Copy, Clone)]
struct Wrapper<T> {
  value: T,
  random_key: u64,
}

impl<T: Cmov + Copy> Cmov for Wrapper<T> {
  fn cmov(&mut self, other: &Self, cond: bool) {
    self.random_key.cmov(&other.random_key, cond);
    self.value.cmov(&other.value, cond);
  }
}

impl<T> Ord for Wrapper<T> {
  fn cmp(&self, other: &Self) -> Ordering {
    self.random_key.cmp(&other.random_key)
  }
}

impl<T> Eq for Wrapper<T> {}

impl<T> PartialOrd for Wrapper<T> {
  fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
    Some(self.cmp(other))
  }
}

impl<T> PartialEq for Wrapper<T> {
  fn eq(&self, other: &Self) -> bool {
    self.random_key == other.random_key
  }
}

fn wrap_data<T, C>(arr: &C) -> Vec<Wrapper<T>>
where
  T: Copy,
  C: Indexable<T>,
{
  let mut wrapped = Vec::with_capacity(arr.len());
  for i in 0..arr.len() {
    wrapped.push(Wrapper { value: arr[i], random_key: random::<u64>() });
  }
  wrapped
}

fn basic_shuffle_inner<T, C>(arr: &mut C, wrapped: &mut Vec<Wrapper<T>>)
where
  T: Cmov + Copy,
  C: Indexable<T>,
{
  bitonic_sort(wrapped);
  for i in 0..arr.len() {
    arr[i] = wrapped[i].value;
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_basic_shuffle() {
    for sz in [100, 1000, 10000] {
      let mut arr: Vec<u32> = (0..sz as u32).collect();
      let mut mark = 0;
      println!("arr: {:?}", arr);
      basic_shuffle(&mut arr);
      println!("arr: {:?}", arr);
      assert_eq!(arr.len(), sz);
      for (i, v) in arr.iter().enumerate() {
        if *v != i as u32 {
          mark = 1;
        }
        assert_eq!(mark, 1);
      }
    }
  }
}
