//! Assembly implementations of the `Cmov` trait.
//!
use bytemuck::{bytes_of, bytes_of_mut, Pod};
use std::arch::asm;

use crate::traits::Cmov;

#[allow(missing_docs)]
pub trait _Cmovbase {
  fn cmov_base(&mut self, other: &Self, choice: bool);
}

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

impl<T> Cmov for T
where
  T: Pod, // Ensure the type is plain old data
{
  #[inline]
  fn cmov(&mut self, other: &Self, choice: bool) {
    let self_bytes = bytes_of_mut(self);
    let other_bytes = bytes_of(other);

    // Process in chunks of u64 first
    let mut i = 0;
    while i + 8 <= self_bytes.len() {
      let self_chunk = &mut self_bytes[i..i + 8];
      let other_chunk = &other_bytes[i..i + 8];

      let self_u64 = u64::from_ne_bytes(self_chunk.try_into().unwrap());
      let other_u64 = u64::from_ne_bytes(other_chunk.try_into().unwrap());

      let mut result = self_u64;
      result.cmov_base(&other_u64, choice);

      self_chunk.copy_from_slice(&result.to_ne_bytes());
      i += 8;
    }

    // Process remaining u32
    while i + 4 <= self_bytes.len() {
      let self_chunk = &mut self_bytes[i..i + 4];
      let other_chunk = &other_bytes[i..i + 4];

      let self_u32 = u32::from_ne_bytes(self_chunk.try_into().unwrap());
      let other_u32 = u32::from_ne_bytes(other_chunk.try_into().unwrap());

      let mut result = self_u32;
      result.cmov_base(&other_u32, choice);

      self_chunk.copy_from_slice(&result.to_ne_bytes());
      i += 4;
    }

    // Process remaining u16
    while i + 2 <= self_bytes.len() {
      let self_chunk = &mut self_bytes[i..i + 2];
      let other_chunk = &other_bytes[i..i + 2];

      let self_u16 = u16::from_ne_bytes(self_chunk.try_into().unwrap());
      let other_u16 = u16::from_ne_bytes(other_chunk.try_into().unwrap());

      let mut result = self_u16;
      result.cmov_base(&other_u16, choice);

      self_chunk.copy_from_slice(&result.to_ne_bytes());
      i += 2;
    }

    // Process remaining u8
    if i < self_bytes.len() {
      let self_u8 = &mut self_bytes[i];
      let other_u8 = other_bytes[i];

      self_u8.cmov_base(&other_u8, choice);
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_cmov() {
    for choice in &[false, true] {
      let mut a = 0u64;
      let b = 0x12345678u64;
      a.cmov(&b, *choice);
      assert_eq!(a, if *choice { b } else { 0 });

      let mut a = 0u32;
      let b = 0x12345678u32;
      a.cmov(&b, *choice);
      assert_eq!(a, if *choice { b } else { 0 });

      let mut a = 0u16;
      let b = 0x1234u16;
      a.cmov(&b, *choice);
      assert_eq!(a, if *choice { b } else { 0 });

      let mut a = 0u8;
      let b = 0x12u8;
      a.cmov(&b, *choice);
      assert_eq!(a, if *choice { b } else { 0 });
    }
  }
}
