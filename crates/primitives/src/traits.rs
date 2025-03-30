//! Traits for conditional move and swap operations.
use crate::indexable::Indexable;

#[allow(missing_docs)]
pub trait _Cmovbase {
  fn cmov_base(&mut self, other: &Self, choice: bool);
  fn cxchg_base(&mut self, other: &mut Self, choice: bool);
}

/// A trait for conditionally moving values with constant memory trace.
///
pub trait Cmov: Sized {
  /// Conditionally move `other` into `self` based on `choice`.
  /// @Oblivious
  fn cmov(&mut self, other: &Self, choice: bool);
  /// Conditionally exchange `other` and `self` based on `choice`.
  /// @Oblivious
  fn cxchg(&mut self, other: &mut Self, choice: bool);

  /// Conditionally set `self` to either `val_false` or `val_true` based on `choice`.
  /// @Oblivious
  #[inline]
  fn cset(&mut self, val_false: &Self, val_true: &Self, choice: bool) {
    self.cmov(val_true, choice);
    self.cmov(val_false, !choice);
  }
}

/// A trait for conditionally swapping values with constant memory trace.
///
#[inline]
pub fn cswap<T: Cmov + Copy>(first: &mut T, second: &mut T, choice: bool) {
  let tmp = *first;
  first.cmov(second, choice);
  second.cmov(&tmp, choice);
}

/// Adds cswap for Indexables of cswap-able types.
pub trait CswapIndex<T> {
  /// Conditionally swap the elements at `i` and `j` based on `choice`.
  /// @Oblivious
  fn cswap(&mut self, i: usize, j: usize, choice: bool);
}

impl<T, C> CswapIndex<T> for C
where
  C: Indexable<T>,
  T: Cmov + Copy,
{
  fn cswap(&mut self, i: usize, j: usize, choice: bool) {
    let mut left = self[i];
    let mut right = self[j];
    cswap(&mut left, &mut right, choice);
    self[i] = left;
    self[j] = right;
  }
}
