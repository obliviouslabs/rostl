#![cfg(any(target_arch = "x86", target_arch = "x86_64"))]

//! x86 assembly implementations of the `Cmov` trait.
//!
use std::arch::asm;

use crate::traits::{Cmov, _Cmovbase};


impl _Cmovbase for u64 {
  #[inline]
  fn cmov_base(&mut self, other: &Self, choice: bool) {
    unsafe {
      asm!(
        "test {mcond}, {mcond}",
        "cmovnz {i1}, {i2}",
        i1 = inout(reg) *self,
        i2 = in(reg) *other,
        mcond = in(reg) choice as Self,
        options(pure,nomem,nostack)
      );
    }
  }
}

impl _Cmovbase for u32 {
  #[inline]
  fn cmov_base(&mut self, other: &Self, choice: bool) {
    unsafe {
      asm!(
        "test {mcond}, {mcond}",
        "cmovnz {i1:e}, {i2:e}",
        i1 = inout(reg) *self,
        i2 = in(reg) *other,
        mcond = in(reg) choice as u64,
        options(pure,nomem,nostack)
      );
    }
  }
}

impl _Cmovbase for u16 {
  #[inline]
  fn cmov_base(&mut self, other: &Self, choice: bool) {
    unsafe {
      asm!(
        "test {mcond}, {mcond}",
        "cmovnz {i1:x}, {i2:x}",
        i1 = inout(reg) *self,
        i2 = in(reg) *other,
        mcond = in(reg) choice as u64,
        options(pure,nomem,nostack)
      );
    }
  }
}

impl _Cmovbase for u8 {
  #[inline]
  fn cmov_base(&mut self, other: &Self, choice: bool) {
    let mut su16 = *self as u16;
    let ou16 = *other as u16;
    su16.cmov(&ou16, choice);
    *self = su16 as Self;
  }
}

