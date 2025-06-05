//! Implements an option type with constant-time conditional move operations.

use crate::traits::Cmov;
use bytemuck::{Pod, Zeroable};

/// An alternative option implementation that is easier to use in constant-time algorithms.
/// This type is designed to be used in scenarios where you need to conditionally move
/// between two values without leaking information about which value is present.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Zeroable)]
pub struct OOption<T>
where
  T: Cmov + Pod + Zeroable,
{
  /// The underlying value of the option.
  pub value: T,
  /// A boolean flag indicating whether the option contains a value.
  pub is_some: bool,
}

impl<T> OOption<T>
where
  T: Cmov + Pod + Zeroable,
{
  /// Creates a new `OOption` with the given value and presence flag.
  pub const fn new(value: T, is_some: bool) -> Self {
    Self { value, is_some }
  }

  /// Returns whether the option contains a value.
  pub const fn is_some(&self) -> bool {
    self.is_some
  }

  /// Returns `value` if `is_some()`, otherwise panics.
  pub fn unwrap(&self) -> T {
    assert!(self.is_some(), "Called `unwrap` on an `OOption` that is `None`.");
    let mut ret = T::zeroed();
    ret.cmov(&self.value, self.is_some);
    ret
  }
}

impl<T> OOption<T>
where
  T: Cmov + Pod + Zeroable + Default,
{
  /// Returns the contained value if present, otherwise returns a default value.
  pub fn unwrap_or_default(&self) -> T {
    let mut ret = T::default();
    ret.cmov(&self.value, self.is_some);
    ret
  }
}

impl<T> Cmov for OOption<T>
where
  T: Cmov + Pod + Zeroable,
{
  fn cmov(&mut self, other: &Self, choice: bool) {
    self.value.cmov(&other.value, choice);
    self.is_some.cmov(&other.is_some, choice);
  }

  fn cxchg(&mut self, other: &mut Self, choice: bool) {
    let tmp_value = self.value;
    self.value.cmov(&other.value, choice);
    other.value.cmov(&tmp_value, choice);

    let tmp_is_some = self.is_some;
    self.is_some.cmov(&other.is_some, choice);
    other.is_some.cmov(&tmp_is_some, choice);
  }
}
