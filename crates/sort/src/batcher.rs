//! Batcher Odd Even Merge Sorter
use rods_primitives::{
  indexable::Indexable,
  traits::{Cmov, CswapIndex},
};
use std::cmp::min;

/// Sorts the given array using the batcher odd even merge sort algorithm.
/// # Arguments
/// * `arr` - A mutable reference to an array that implements the `Indexable` trait.
/// # Oblivious
/// * Data-independent memory access pattern
/// * Leaks: `arr.len()`
/// # Type Parameters
/// * `T` - The type of the elements in the array. Must implement `Ord`, `Cmov`, and `Copy`.
/// * `C` - The type of the container. Must implement `Indexable<T>`.
///
/// Uses the implementation from <https://ieeexplore.ieee.org/document/8478515>
/// `UNDONE()`: I didn't have the time to read the paper and analyze if the transformations are ok
#[deprecated(note = "I'm not sure if this paper is correct. Don't use this function")]
fn _batcher_sort_untrusted_but_faster<T, C>(arr: &mut C)
where
  T: Ord + Cmov + Copy,
  C: Indexable<T>,
{
  // Undone: optimize this code
  let n = arr.len();
  let mut p = 1;
  while p < n {
    let mut k = p;
    while k > 0 {
      let mut j = k & (p - 1);
      while j < n - k {
        if (j | (2 * p - 1)) == ((j + k) | (2 * p - 1)) {
          let mut i = min(k, n - j - k);
          while i > 0 {
            i -= 1;
            arr.cswap(j + i, j + i + k, arr[j + i] > arr[j + i + k]);
          }
        }
        j += 2 * k;
      }
      k /= 2;
    }
    p *= 2;
  }
}

/// Sorts the given array using the batcher odd even merge sort algorithm.
/// # Arguments
/// * `arr` - A mutable reference to an array that implements the `Indexable` trait.
/// # Oblivious
/// * Data-independent memory access pattern
/// * Leaks: `arr.len()`
/// * `T` - The type of the elements in the array. Must implement `Ord`, `Cmov`, and `Copy`.
/// * `C` - The type of the container. Must implement `Indexable<T>`.
///
/// Uses the original paper implementation.
#[deprecated(note = "please use `bitonic_sort` instead, this is slower")]
pub fn batcher_sort_paper<T, C>(arr: &mut C)
where
  T: Ord + Cmov + Copy,
  C: Indexable<T>,
{
  let n = arr.len();
  let mut p = 1;
  while p < n {
    let mut k = p;
    while k > 0 {
      let mut j = k % p;
      while j < n - k {
        for i in 0..min(k, n - j - k) {
          if ((i + j) / (p * 2)) == ((i + j + k) / (p * 2)) {
            arr.cswap(i + j, i + j + k, arr[i + j] > arr[i + j + k]);
          }
        }
        j += 2 * k;
      }
      k /= 2;
    }
    p *= 2;
  }
}

/// Sorts the given array using the batcher odd even merge sort algorithm.
/// # Arguments
/// * `arr` - A mutable reference to an array that implements the `Indexable` trait.
/// # Oblivious
/// * Data-independent memory access pattern
/// * Leaks: `arr.len()`
/// # Type Parameters
/// * `T` - The type of the elements in the array. Must implement `Ord`, `Cmov`, and `Copy`.
/// * `C` - The type of the container. Must implement `Indexable<T>`.
#[deprecated(note = "please use `bitonic_sort` instead, this is slower")]
pub fn batcher_sort<T, C>(arr: &mut C)
where
  T: Ord + Cmov + Copy,
  C: Indexable<T>,
{
  #[allow(deprecated)]
  batcher_sort_paper(arr)
}

#[cfg(test)]
#[allow(deprecated)]
mod tests {
  use super::*;
  use rand::random;

  #[test]
  fn test_batcher_sort() {
    for sz in 0..42 {
      let mut arr: Vec<u32> = (0..sz as u32).collect();
      for i in 0..sz {
        let j = random::<usize>() % sz;
        arr.swap(i, j);
      }
      batcher_sort(&mut arr);
      assert_eq!(arr.len(), sz);
      for (i, v) in arr.iter().enumerate() {
        assert_eq!(*v, i as u32);
      }
    }
  }

  #[test]
  fn test_batcher_sort_large() {
    let mut arr: Vec<u32> = (0..1000).rev().collect();
    batcher_sort(&mut arr);
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
    batcher_sort(&mut arr);
    assert_eq!(arr.len(), sz);
    for (i, v) in arr.iter().enumerate() {
      assert_eq!(*v, i as u32);
    }
  }
}
