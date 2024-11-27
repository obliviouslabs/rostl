//! Bose-Nelson sorting network.
use rods_primitives::{
  indexable::Indexable,
  traits::{Cmov, CswapIndex},
};

use crate::CSWAP;

/// Bose-Nelson network to merge two sorted arrays.
/// # Arguments
/// * `arr` - A mutable reference to an array that implements the `Indexable` trait.
/// * `start1` - The starting index of the first subarray.
/// * `size1` - The size of the first subarray.
/// * `start2` - The starting index of the second subarray.
/// * `size2` - The size of the second subarray.
/// # Type Parameters
/// * `T` - The type of the elements in the array. Must implement `Ord`, `Cmov`, and `Copy`.
/// * `C` - The type of the container. Must implement `Indexable<T>`.
pub fn bn_merge<T, C>(arr: &mut C, start1: usize, size1: usize, start2: usize, size2: usize)
where
  T: Ord + Cmov + Copy,
  C: Indexable<T>,
{
  debug_assert!(start1 + size1 <= start2);
  debug_assert!(start2 + size2 <= arr.len());
  if size1 == 1 && size2 == 1 {
    CSWAP!(arr, start1, start2);
  } else if size1 == 1 && size2 == 2 {
    CSWAP!(arr, start1, start2 + 1);
    CSWAP!(arr, start1, start2);
  } else if size1 == 2 && size2 == 1 {
    CSWAP!(arr, start1, start2);
    CSWAP!(arr, start1 + 1, start2);
  } else {
    let s1 = size1 / 2;
    let s2 = if (size1 % 2) == 0 { (size2 + 1) / 2 } else { size2 / 2 };
    bn_merge(arr, start1, s1, start2, s2);
    bn_merge(arr, start1 + s1, size1 - s1, start2 + s2, size2 - s2);
    bn_merge(arr, start1 + s1, size1 - s1, start2, s2);
  }
}

/// Recursively sorts the array using the Bose-Nelson sorting network.
/// # Arguments
/// * `arr` - A mutable reference to an array that implements the `Indexable` trait.
/// * `start` - The starting index of the section to sort.
/// * `size` - The size of the section to sort.
/// # Type Parameters
/// * `T` - The type of the elements in the array. Must implement `Ord`, `Cmov`, and `Copy`.
/// * `C` - The type of the container. Must implement `Indexable<T>`.
pub fn bn_sort<T, C>(arr: &mut C, start: usize, size: usize)
where
  T: Ord + Cmov + Copy,
  C: Indexable<T>,
{
  if size <= 1 {
    return;
  }

  let s2 = size / 2;
  bn_sort(arr, start, s2);
  bn_sort(arr, start + s2, size - s2);
  bn_merge(arr, start, s2, start + s2, size - s2);
}

/// Sorts the given array using the bose-nelson sorting network.
/// # Arguments
/// * `arr` - A mutable reference to an array that implements the `Indexable` trait.
/// # Oblivious
/// * Data-independent memory access pattern
/// * Leaks: `arr.len()`
/// * `T` - The type of the elements in the array. Must implement `Ord`, `Cmov`, and `Copy`.
/// * `C` - The type of the container. Must implement `Indexable<T>`.
///
#[deprecated(note = "please use `bitonic_sort` instead, this is slower")]
pub fn bose_nelson_sort<T, C>(arr: &mut C)
where
  T: Ord + Cmov + Copy,
  C: Indexable<T>,
{
  bn_sort::<T, C>(arr, 0, arr.len());
}

#[cfg(test)]
#[allow(deprecated)]
mod tests {
  use super::*;
  use rand::random;

  #[test]
  fn test_bose_nelson_sort() {
    for sz in 0..42 {
      let mut arr: Vec<u32> = (0..sz as u32).collect();
      for i in 0..sz {
        let j = random::<usize>() % sz;
        arr.swap(i, j);
      }
      bose_nelson_sort(&mut arr);
      assert_eq!(arr.len(), sz);
      for (i, v) in arr.iter().enumerate() {
        assert_eq!(*v, i as u32);
      }
    }
  }

  #[test]
  fn test_bose_nelson_sort_large() {
    let mut arr: Vec<u32> = (0..1000).rev().collect();
    bose_nelson_sort(&mut arr);
    assert_eq!(arr.len(), 1000);
    for (i, v) in arr.iter().enumerate() {
      assert_eq!(*v, i as u32);
    }

    let sz = random::<usize>() % 1000;
    let mut arr: Vec<u32> = (0..sz as u32).rev().collect();
    // random permutation:
    for i in 0..sz {
      let j = random::<usize>() % sz;
      arr.swap(i, j);
    }
    bose_nelson_sort(&mut arr);
    assert_eq!(arr.len(), sz);
    for (i, v) in arr.iter().enumerate() {
      assert_eq!(*v, i as u32);
    }
  }
}
