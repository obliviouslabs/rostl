//! Some common utility functions that should be in the standard library.
//! This is a temporary solution until they are added to the standard library.

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
