//! Basic shuffling algorithm (Tag + sort based).
use crate::bitonic::bitonic_sort;
use rand::Rng;
use rods_primitives::indexable::Indexable;
use rods_primitives::traits::Cmov;
use std::cmp::Ordering;

/// Does a random shuffle of `arr` by adding a random tag to each element and sorting based on that tag.
pub fn shuffle<T, C>(arr: &mut C)
where
  T: Cmov + Copy,
  C: Indexable<T>,
{
  let mut wrapped = wrap_data(arr);
  bitonic_sort(&mut wrapped);
  unwrap_data(arr, &wrapped);
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
  fn cxchg(&mut self, other: &mut Self, cond: bool) {
    let c = *self;
    self.cmov(other, cond);
    other.cmov(&c, cond);
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
  let mut rng = rand::rng();
  for i in 0..arr.len() {
    wrapped.push(Wrapper { value: arr[i], random_key: rng.random::<u64>() });
  }
  wrapped
}

fn unwrap_data<T, C>(arr: &mut C, wrapped: &[Wrapper<T>])
where
  T: Copy,
  C: Indexable<T>,
{
  for i in 0..arr.len() {
    arr[i] = wrapped[i].value;
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_shuffle() {
    for sz in [100, 1000, 10000] {
      let mut arr: Vec<u32> = (0..sz as u32).collect();
      let mut mark = 0;
      println!("arr: {:?}", arr);
      shuffle(&mut arr);
      println!("arr: {:?}", arr);
      assert_eq!(arr.len(), sz);
      arr.sort();
      for (i, v) in arr.iter().enumerate() {
        if *v != i as u32 {
          mark = 1;
        }
        assert_eq!(mark, 0);
      }
    }
  }
  //UNDONE(git-27): Add more tests to run a huge number of shuffles and check if each element has a similar probability at any index.
}
