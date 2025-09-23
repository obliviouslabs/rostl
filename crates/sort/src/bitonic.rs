//! Bitonic Sorter
#![allow(unused_macros)]

use rostl_primitives::indexable::Indexable;
use rostl_primitives::traits::{Cmov, CswapIndex};

use assume::assume;
use rostl_primitives::utils::get_strictly_bigger_power_of_two;

macro_rules! __bitonic_merge_pow2_body
{
  (
    $arr:ident, $start:ident, $size:ident,
    $swap_function: ident
  ) => {{
    assume!(unsafe: $size > 1);
    assume!(unsafe: $start + $size <= $arr.len());
    assume!(unsafe: $size.is_power_of_two());

    let l2 = $size.trailing_zeros();
    for sz in 0..l2 {
      let blocks = 1 << sz;
      let s2 = $size >> (sz + 1);
      for j in 0..blocks {
        for i in 0..s2 {
          let left = $start + 2 * j * s2 + i;
          let right = left + s2;
          assume!(unsafe: left < $arr.len());
          assume!(unsafe: right < $arr.len());
          let cond = $arr[left] <= $arr[right];
          if UP {
            $swap_function!(left, right, !cond);
          } else {
            $swap_function!(left, right, cond);
          }
        }
      }
    }
  }};
}

macro_rules! __bitonic_merge_body
{
  (
    $arr:ident, $start:ident, $size:ident,
    $swap_function: ident,
    $merge_pow2_call: ident,
    $merge_call: ident
  ) => {{
    assume!(unsafe: $size > 1);
    assume!(unsafe: $start + $size <= $arr.len());

    let p2 = get_strictly_bigger_power_of_two($size - 1);
    let s2: usize = p2 / 2;

    for i in 0..($size - s2) {
      let left = $start + i;
      let right = left + s2;
      assume!(unsafe: left < $arr.len());
      assume!(unsafe: right < $arr.len());
      let cond = $arr[left] <= $arr[right];
      if UP {
        $swap_function!(left, right, !cond);
      } else {
        $swap_function!(left, right, cond);
      }
    }

    if s2 > 1 {
      merge_pow2_call!($start, s2);
    }

    if $size - s2 > 1 {
      merge_call!($start + s2, $size - s2);
    }
  }};
}

macro_rules! __bitonic_sort_inner_body
{
  (
    $arr:ident, $start:ident, $size:ident,
    $sort_inner_call: ident,
    $merge_call: ident
  ) => {{
    assume!(unsafe: $size > 1);
    assume!(unsafe: $start + $size <= $arr.len());

    let half_size = $size / 2;

    if $size - half_size > 1 {
      $sort_inner_call!($start + half_size, $size - half_size, UP, DOWN);
      if half_size > 1 {
        $sort_inner_call!($start, half_size, DOWN, UP);
      }
    }

    $merge_call!($start, $size);
  }};
}

#[rustfmt::skip]
macro_rules! bitonic_expressions {
  ($arr: ident) => {
    macro_rules! swap_function {
      ($left:ident, $right:ident, $cond:expr) => {
        assume!(unsafe: $left < $arr.len());
        assume!(unsafe: $right < $arr.len());
        $arr.cswap($left, $right, $cond);
      };
    }
    macro_rules! merge_pow2_call {
      ($start:expr, $size:expr) => {{
        bitonic_merge_pow2::<T, C, UP>($arr, $start, $size);
      }};
    }
    macro_rules! merge_call {
      ($start:expr, $size:expr) => {{
        bitonic_merge::<T, C, UP>($arr, $start, $size);
      }};
    }
    macro_rules! sort_inner_call {
      ($start:expr, $size:expr, $UP:ident, $DOWN:ident) => {{
        bitonic_sort_inner::<T, C, $UP, $DOWN>($arr, $start, $size);
      }};
    }
  };
}

fn bitonic_merge<T, C, const UP: bool>(arr: &mut C, start: usize, size: usize)
where
  T: Ord + Cmov + Copy,
  C: Indexable<T>,
{
  bitonic_expressions!(arr);
  __bitonic_merge_body!(arr, start, size, swap_function, merge_pow2_call, merge_call);
}

#[inline(always)]
fn bitonic_merge_pow2<T, C, const UP: bool>(arr: &mut C, start: usize, size: usize)
where
  T: Ord + Cmov + Copy,
  C: Indexable<T>,
{
  bitonic_expressions!(arr);
  __bitonic_merge_pow2_body!(arr, start, size, swap_function);
}

fn bitonic_sort_inner<T, C, const UP: bool, const DOWN: bool>(
  arr: &mut C,
  start: usize,
  size: usize,
) where
  T: Ord + Cmov + Copy,
  C: Indexable<T>,
{
  bitonic_expressions!(arr);
  __bitonic_sort_inner_body!(arr, start, size, sort_inner_call, merge_call);
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
  if arr.len() <= 1 {
    return;
  }
  bitonic_sort_inner::<T, C, true, false>(arr, 0, arr.len());
}

#[rustfmt::skip]
macro_rules! bitonic_payload_expressions {
  ($arr: ident, $payload: ident) => {
    macro_rules! swap_function {
      ($left:ident, $right:ident, $cond:expr) => {
        assume!(unsafe: $left < $arr.len());
        assume!(unsafe: $right < $arr.len());
        assume!(unsafe: $left < $payload.len());
        assume!(unsafe: $right < $payload.len());
        assume!(unsafe: $arr.len() == $payload.len());
        $arr.cswap($left, $right, $cond);
        $payload.cswap($left, $right, $cond);
      };
    }
    macro_rules! merge_pow2_call {
      ($start:expr, $size:expr) => {
        bitonic_payload_merge_pow2::<T, C, P, UP>($arr, $payload, $start, $size);
      };
    }
    macro_rules! merge_call {
      ($start:expr, $size:expr) => {
        bitonic_payload_merge::<T, C, P, UP>($arr, $payload, $start, $size);
      };
    }
    macro_rules! sort_inner_call {
      ($start:expr, $size:expr, $UP:ident, $DOWN:ident) => {
        bitonic_payload_sort_inner::<T, C, P, $UP, $DOWN>($arr, $payload, $start, $size);
      };
    }
  };
}

fn bitonic_payload_merge<T, C, P, const UP: bool>(
  arr: &mut C,
  payload: &mut [P],
  start: usize,
  size: usize,
) where
  T: Ord + Cmov + Copy,
  P: Cmov + Copy,
  C: Indexable<T> + ?Sized,
{
  bitonic_payload_expressions!(arr, payload);
  __bitonic_merge_body!(arr, start, size, swap_function, merge_pow2_call, merge_call);
}

#[inline(always)]
fn bitonic_payload_merge_pow2<T, C, P, const UP: bool>(
  arr: &mut C,
  payload: &mut [P],
  start: usize,
  size: usize,
) where
  T: Ord + Cmov + Copy,
  P: Cmov + Copy,
  C: Indexable<T> + ?Sized,
{
  bitonic_payload_expressions!(arr, payload);
  __bitonic_merge_pow2_body!(arr, start, size, swap_function);
}

fn bitonic_payload_sort_inner<T, C, P, const UP: bool, const DOWN: bool>(
  arr: &mut C,
  payload: &mut [P],
  start: usize,
  size: usize,
) where
  T: Ord + Cmov + Copy,
  P: Cmov + Copy,
  C: Indexable<T> + ?Sized,
{
  bitonic_payload_expressions!(arr, payload);
  __bitonic_sort_inner_body!(arr, start, size, sort_inner_call, merge_call);
}

/// Sorts the given array using the bitonic sort algorithm.
/// # Arguments
/// * `arr` - A mutable reference to the array to be sorted.
/// * `payload` - A mutable reference to the payload array to be rearranged alongside the main array.
/// # Requires
/// * `arr.len() == payload.len()`
/// # Oblivious
/// * Data-independent memory access pattern.
/// * Leaks: `arr.len()`
/// # Type Parameters
/// * `T` - The type of the elements in the array. Must implement `Ord`, `Cmov`, and `Copy`.
/// * `C` - The type of the container. Must be `Indexable<T>`.
/// * `P` - The type of the payload container. Must be `Indexable<T>`.
///
pub fn bitonic_payload_sort<T, C, P>(arr: &mut C, payload: &mut [P])
where
  T: Ord + Cmov + Copy,
  P: Cmov + Copy,
  C: Indexable<T> + ?Sized,
{
  if arr.len() <= 1 {
    return;
  }
  assert!(arr.len() == payload.len());
  assume!(unsafe: arr.len() == payload.len());
  bitonic_payload_sort_inner::<T, C, P, true, false>(arr, payload, 0, arr.len());
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

  #[test]
  fn test_bitonic_payload_sort() {
    for sz in 0..42 {
      let mut arr: Vec<u32> = (0..sz as u32).collect();
      let mut payload: Vec<u32> = (1000..1000 + sz as u32).collect();

      for i in 0..sz {
        let j = rand::rng().random_range(0..sz);
        arr.swap(i, j);
        payload.swap(i, j);
      }

      bitonic_payload_sort(&mut arr, &mut payload);

      assert_eq!(arr.len(), sz);
      assert_eq!(payload.len(), sz);
      for (i, v) in arr.iter().enumerate() {
        assert_eq!(*v, i as u32);
        assert_eq!(payload[i], 1000 + i as u32);
      }
    }
  }
}
