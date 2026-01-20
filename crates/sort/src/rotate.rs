//! Implements oblivious shift and rotation algorithms.
//!

use assume::assume;
use rostl_primitives::traits::{Cmov, CswapIndex};

/// Reverses the region arr[start..end) in-place using oblivious conditional swaps.
///
/// # Behavior
/// * If `enable` is true, the region arr[start..end) is reversed in-place.
/// * If `enable` is false, the function has no effects.
///
/// # Oblivious
/// * The function reveals `start` and `end`.
fn reverse_region<T>(arr: &mut [T], start: usize, end: usize, enable: bool)
where
  T: Cmov + Copy,
{
  let len = end - start;
  let half = len >> 1;
  for i in 0..half {
    let a = start + i;
    let b = end - 1 - i;
    assume!(unsafe: a < arr.len());
    assume!(unsafe: b < arr.len());
    // perform the pairwise swap only when `enable` is true; addresses touched are independent of `enable`.
    arr.cswap(a, b, enable);
  }
}

/// Obliviously rotates `arr` left by `k` positions in-place.
///
/// # Behavior
/// * After calling `rotate_left(arr, k)`, element originally at index `i` will be at index `(i + n - k) % n` (left rotate by k).
///
/// # Oblivious
/// * Memory access pattern depends only on `arr.len()`.
pub fn rotate_left<T>(arr: &mut [T], k: usize)
where
  T: Cmov + Copy,
{
  let n = arr.len();
  if n <= 1 {
    return;
  }
  assert!(k <= n, "rotate_left: k must be less than arr.len()");
  let mut s = 1;

  // Iterate over powers of two `s` (1,2,4,...) less than n. For each `s` we execute the reversal
  // pattern that would rotate by `s`, but we only actually swap when `(s & k) != 0`.
  while s < n {
    let enable = (s & k) != 0;
    reverse_region(arr, 0, s, enable);
    reverse_region(arr, s, n, enable);
    reverse_region(arr, 0, n, enable);
    s <<= 1;
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use rand::Rng;

  #[test]
  fn test_rotate_left_basic() {
    let mut arr = [1, 2, 3, 4, 5];
    rotate_left(&mut arr, 2);
    assert_eq!(&arr, &[3, 4, 5, 1, 2]);

    let mut arr = [1, 2, 3, 4, 5];
    rotate_left(&mut arr, 5);
    assert_eq!(&arr, &[1, 2, 3, 4, 5]);

    let mut arr = [1];
    rotate_left(&mut arr, 1);
    assert_eq!(&arr, &[1]);
  }

  #[test]
  fn test_rotate_left_edge() {
    let mut arr: [u8; 0] = [];
    rotate_left(&mut arr, 0);
    assert_eq!(&arr, &[]);

    let mut arr = [42];
    rotate_left(&mut arr, 0);
    assert_eq!(&arr, &[42]);
    rotate_left(&mut arr, 1);
    assert_eq!(&arr, &[42]);

    let mut arr = [1, 2];
    rotate_left(&mut arr, 0);
    assert_eq!(&arr, &[1, 2]);
    rotate_left(&mut arr, 1);
    assert_eq!(&arr, &[2, 1]);
    rotate_left(&mut arr, 2);
    assert_eq!(&arr, &[2, 1]);

    let mut arr = [1, 2, 3];
    rotate_left(&mut arr, 0);
    assert_eq!(&arr, &[1, 2, 3]);
    rotate_left(&mut arr, 1);
    assert_eq!(&arr, &[2, 3, 1]);
    rotate_left(&mut arr, 2);
    assert_eq!(&arr, &[1, 2, 3]);
    rotate_left(&mut arr, 3);
    assert_eq!(&arr, &[1, 2, 3]);
  }

  #[test]
  fn test_rotate_left_random() {
    let mut rng = rand::rng();
    for _ in 0..100 {
      let size = rng.random_range(0..2050);
      let arr: Vec<i32> = (0..size).map(|_| rng.random_range(0..100)).collect();
      let mut arr1 = arr.clone();
      let k = if size == 0 { 0 } else { rng.random_range(0..size) };
      rotate_left(&mut arr1, k);
      let mut expected = Vec::with_capacity(size);
      if size > 0 {
        expected.extend_from_slice(&arr[k..]);
        expected.extend_from_slice(&arr[..k]);
      }
      assert_eq!(arr1, expected);
    }
  }
}
