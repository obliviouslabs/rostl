//! Implements oblivious compaction algorithms.
//!

use rods_primitives::traits::{Cmov, CswapIndex};
// use rods_primitives::indexable::{Indexable, Length};

/// Stably compacts an array `arr` of length n in place using nlogn oblivious compaction.
/// Uses `https://arxiv.org/pdf/1103.5102`
/// # Behavior
/// * returns `ret` - the number of non-dummy elements (new real length of the array).
/// * first `ret` elements of `arr` are the non-dummy elements in the same order as they were in the original array.
/// * the rest of the elements in `arr` are dummy elements.
/// # Oblivious
/// * Fully data-independent memory access pattern.
/// * Leaks: `arr.len()` - the full length of the original array
pub fn compact<T, F>(arr: &mut [T], is_dummy: F) -> usize
where
  F: Fn(&T) -> bool,
  T: Cmov + Copy,
{
  if arr.is_empty() {
    return 0;
  }
  let l2len = arr.len().next_power_of_two().trailing_zeros() as usize;
  let mut csum = vec![0; arr.len()];
  let mut dummy_count = 0;

  csum[0] = 0;
  let pred = is_dummy(&arr[0]);
  dummy_count.cmov(&1, pred);
  for i in 1..arr.len() {
    csum[i] = 0;
    let pred = is_dummy(&arr[i]);
    dummy_count.cmov(&(dummy_count + 1), pred);
    csum[i].cmov(&(dummy_count), !pred);
  }
  let ret = arr.len() - dummy_count;

  for i in 0..l2len {
    let offset = 1 << i;
    for j in 0..(arr.len() - offset) {
      let a = j;
      let b = j + offset;
      let pred = (csum[b] & offset) != 0;
      arr.cswap(a, b, pred);
      let newacsum = csum[b].wrapping_sub(offset);
      csum[a].cmov(&newacsum, pred);
      csum[b].cmov(&0, pred);
    }
  }

  ret
}

#[cfg(test)]
mod tests {
  use rand::Rng;

  use super::*;

  #[test]
  fn test_compact() {
    let mut arr = [1, 2, 3, 4, 5];
    let new_len = compact(&mut arr, |x| *x % 2 == 0);
    assert_eq!(new_len, 3);
    assert_eq!(&arr[..new_len], &[1, 3, 5]);
  }

  #[test]
  fn test_small() {
    let mut arr: Vec<i32> = vec![1];
    let new_len = compact(&mut arr, |x| *x % 2 == 0);
    assert_eq!(new_len, 1);
    assert_eq!(&arr[..new_len], &[1]);

    let mut arr: Vec<i32> = vec![2];
    let new_len = compact(&mut arr, |x| *x % 2 == 0);
    assert_eq!(new_len, 0);
    assert_eq!(&arr[..new_len], &[]);

    let mut arr: Vec<i32> = vec![1, 2];
    let new_len = compact(&mut arr, |x| *x % 2 == 0);
    assert_eq!(new_len, 1);
    assert_eq!(&arr[..new_len], &[1]);

    let mut arr: Vec<i32> = vec![];
    let new_len = compact(&mut arr, |x| *x % 2 == 0);
    assert_eq!(new_len, 0);
    assert_eq!(&arr[..new_len], &[]);
  }

  #[test]
  fn test_many_sizes() {
    // Picks a random array size and fills with random values and checks if it's correct via a non oblivious comparison
    let mut rng = rand::rng();
    for _i in 0..100 {
      let size = rng.random_range(0..2050);
      let mut arr: Vec<i32> = (0..size).map(|_| rng.random_range(0..100)).collect();
      let new_len = compact(&mut arr, |x| *x % 2 == 0);
      for itm in arr.iter().take(new_len) {
        assert!(itm % 2 != 0);
      }
      for itm in arr.iter().skip(new_len) {
        assert!(itm % 2 == 0);
      }
    }
  }
}
