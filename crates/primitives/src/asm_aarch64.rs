#![cfg(target_arch = "aarch64")]
#![deprecated(
  since = "1.0.0",
  note = "Support for aarch64 is not trace oblivious yet, feel free to complete this file."
)]

//! Assembly implementations of the `Cmov` trait.
//!
// use std::arch::asm;

use crate::traits::_Cmovbase;

impl _Cmovbase for u64 {
  #[inline]
  fn cmov_base(&mut self, other: &Self, choice: bool) {
    if choice {
      *self = *other
    }
  }
  #[inline]
  fn cxchg_base(&mut self, other: &mut Self, choice: bool) {
    let c = *self;
    self.cmov_base(other, choice);
    other.cmov_base(&c, choice);
  }
}

impl _Cmovbase for u32 {
  #[inline]
  fn cmov_base(&mut self, other: &Self, choice: bool) {
    if choice {
      *self = *other
    }
  }
  #[inline]
  fn cxchg_base(&mut self, other: &mut Self, choice: bool) {
    let c = *self;
    self.cmov_base(other, choice);
    other.cmov_base(&c, choice);
  }
}

impl _Cmovbase for u16 {
  #[inline]
  fn cmov_base(&mut self, other: &Self, choice: bool) {
    if choice {
      *self = *other
    }
  }
  #[inline]
  fn cxchg_base(&mut self, other: &mut Self, choice: bool) {
    let c = *self;
    self.cmov_base(other, choice);
    other.cmov_base(&c, choice);
  }
}

impl _Cmovbase for u8 {
  #[inline]
  fn cmov_base(&mut self, other: &Self, choice: bool) {
    if choice {
      *self = *other
    }
  }
  #[inline]
  fn cxchg_base(&mut self, other: &mut Self, choice: bool) {
    let c = *self;
    self.cmov_base(other, choice);
    other.cmov_base(&c, choice);
  }
}
