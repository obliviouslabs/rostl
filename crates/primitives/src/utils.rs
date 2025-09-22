//! Some common utility functions that should be in the standard library.
//! This is a temporary solution until they are added to the standard library.

use static_assertions::const_assert;

/// Const version of `std::cmp::max`.
pub const fn max(a: usize, b: usize) -> usize {
  if a > b {
    a
  } else {
    b
  }
}

/// Const version of `std::cmp::min`.
pub const fn min(a: usize, b: usize) -> usize {
  if a < b {
    a
  } else {
    b
  }
}

const_assert!(usize::BITS == 64);

/// Returns the smallest power of two that is strictly greater than the given size.
#[inline]
pub const fn get_strictly_bigger_power_of_two(mut n: usize) -> usize {
  debug_assert!(n <= 0x8000_0000_0000_0000);
  n = n | (n >> 1);
  n = n | (n >> 2);
  n = n | (n >> 4);
  n = n | (n >> 8);
  n = n | (n >> 16);
  n = n | (n >> 32);
  n.saturating_add(1)
}

/// Returns the smallest power of two that is <= the given size.
#[inline]
pub const fn get_smaller_or_equal_power_of_two(n: usize) -> usize {
  get_strictly_bigger_power_of_two(n) >> 1
}

/// Returns the smallest power of two that is strictly greater than the given size,
#[deprecated(note = "use get_strictly_bigger_power_of_two instead")]
#[inline]
pub const fn get_strictly_bigger_power_of_two_clz(size: usize) -> usize {
  1 << (usize::BITS - size.leading_zeros()) as usize
}

/// Returns the smallest power of two that is strictly greater than the given size.
#[deprecated(note = "use get_strictly_bigger_power_of_two instead")]
#[inline]
pub const fn get_strictly_bigger_power_of_two_loop(size: usize) -> usize {
  let mut n = 1;
  while n <= size {
    n *= 2;
  }
  n
}

#[cfg(test)]
mod tests {
  use super::*;
  #[test]
  fn test_np2() {
    assert_eq!(get_strictly_bigger_power_of_two(0), 1);
    assert_eq!(get_strictly_bigger_power_of_two(1), 2);
    assert_eq!(get_strictly_bigger_power_of_two(2), 4);
    assert_eq!(get_strictly_bigger_power_of_two(3), 4);
    assert_eq!(get_strictly_bigger_power_of_two(4), 8);
    assert_eq!(get_strictly_bigger_power_of_two(5), 8);
    assert_eq!(get_strictly_bigger_power_of_two(6), 8);
    assert_eq!(get_strictly_bigger_power_of_two(7), 8);
    assert_eq!(get_strictly_bigger_power_of_two(8), 16);
    assert_eq!(get_strictly_bigger_power_of_two(9), 16);
    assert_eq!(get_strictly_bigger_power_of_two(15), 16);
    assert_eq!(get_strictly_bigger_power_of_two(16), 32);
  }

  #[test]
  fn test_smaller_equal_np2() {
    assert_eq!(get_smaller_or_equal_power_of_two(1), 1);
    assert_eq!(get_smaller_or_equal_power_of_two(2), 2);
    assert_eq!(get_smaller_or_equal_power_of_two(3), 2);
    assert_eq!(get_smaller_or_equal_power_of_two(4), 4);
    assert_eq!(get_smaller_or_equal_power_of_two(5), 4);
    assert_eq!(get_smaller_or_equal_power_of_two(6), 4);
    assert_eq!(get_smaller_or_equal_power_of_two(7), 4);
    assert_eq!(get_smaller_or_equal_power_of_two(8), 8);
    assert_eq!(get_smaller_or_equal_power_of_two(9), 8);
    assert_eq!(get_smaller_or_equal_power_of_two(15), 8);
    assert_eq!(get_smaller_or_equal_power_of_two(16), 16);
  }
}
