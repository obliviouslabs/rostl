//! Bitonic Sorter
use rods_primitives::indexable::Indexable;
use rods_primitives::traits::{Cmov, CswapIndex};

use crate::utils::get_strictly_bigger_power_of_two;

fn bitonic_merge_pow2<T, C, const UP: bool>(arr: &mut C, start: usize, size: usize)
where
  T: Ord + Cmov + Copy,
  C: Indexable<T>,
{
  let l2 = size.trailing_zeros();

  for sz in 0..l2 {
    let blocks = 1 << sz;
    let s2 = size >> (sz + 1);
    for j in 0..blocks {
      for i in start + 2 * j * s2..start + (2 * j + 1) * s2 {
        arr.cswap(i, i + s2, (arr[i] > arr[i + s2]) == UP);
      }
    }
  }
}

fn bitonic_merge<T, C, const UP: bool>(arr: &mut C, start: usize, size: usize)
where
  T: Ord + Cmov + Copy,
  C: Indexable<T>,
{
  if size <= 1 {
    return;
  }
  let p2 = get_strictly_bigger_power_of_two(size);
  let s2: usize = p2 / 2;

  for i in start..start + (size - s2) {
    arr.cswap(i, s2 + i, (arr[i] > arr[s2 + i]) == UP);
  }

  bitonic_merge_pow2::<T, C, UP>(arr, start, s2);
  bitonic_merge::<T, C, UP>(arr, start + s2, size - s2);
}

fn bitonic_sort_inner<T, C, const UP: bool, const DOWN: bool>(
  arr: &mut C,
  start: usize,
  size: usize,
) where
  T: Ord + Cmov + Copy,
  C: Indexable<T>,
{
  if size <= 1 {
    return;
  }

  let s2 = size / 2;
  bitonic_sort_inner::<T, C, DOWN, UP>(arr, start, s2);
  bitonic_sort_inner::<T, C, UP, DOWN>(arr, start + s2, size - s2);
  bitonic_merge::<T, C, UP>(arr, start, size);
}

/// Sorts the given array using the bitonic sort algorithm.
/// # Arguments
/// * `arr` - A mutable reference to the array to be sorted.
/// # Oblivious
/// * Data-independent memory access pattern.
/// * Leaks: `arr.len()`
/// # Type Parameters
/// * `T` - The type of the elements in the array. Must implement `Ord`, `Cmov`, and `Copy`.
/// * `C` - The type of the container. Must be `Indexable<T>`.
///
pub fn bitonic_sort<T, C>(arr: &mut C)
where
  T: Ord + Cmov + Copy,
  C: Indexable<T>,
{
  bitonic_sort_inner::<T, C, true, false>(arr, 0, arr.len());
}

#[cfg(test)]
mod tests {
  use super::*;
  use rand::Rng;

  #[test]
  fn test_bitonic_sort() {
    for sz in 0..42 {
      let mut arr: Vec<u32> = (0..sz as u32).collect();
      for i in 0..sz {
        let j = rand::rng().random_range(0..sz);
        arr.swap(i, j);
      }
      bitonic_sort(&mut arr);
      assert_eq!(arr.len(), sz);
      for (i, v) in arr.iter().enumerate() {
        assert_eq!(*v, i as u32);
      }
    }
  }

  #[test]
  fn test_bitonic_sort_large() {
    let mut arr: Vec<u32> = (0..1000).rev().collect();
    bitonic_sort(&mut arr);
    assert_eq!(arr.len(), 1000);
    for (i, v) in arr.iter().enumerate() {
      assert_eq!(*v, i as u32);
    }

    let sz = rand::rng().random_range(0..1000);
    let mut arr: Vec<u32> = (0..sz as u32).rev().collect();
    // random permutation:
    for i in 0..sz {
      let j = rand::rng().random_range(0..sz);
      arr.swap(i, j);
    }
    bitonic_sort(&mut arr);
    assert_eq!(arr.len(), sz);
    for (i, v) in arr.iter().enumerate() {
      assert_eq!(*v, i as u32);
    }
  }
}
