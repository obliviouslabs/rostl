#![cfg(any(target_arch = "x86", target_arch = "x86_64"))]

//! x86 assembly implementations of the `Cmov` trait.
//!
use core::arch::x86_64::*;
use std::arch::asm;

use crate::traits::{Cmov, _Cmovbase};

#[cfg(all(target_feature = "avx512f", target_feature = "avx512vl"))]
impl _Cmovbase for [u8; 64] {
  #[inline]
  fn cmov_base(&mut self, other: &Self, choice: bool) {
    let mask_val: u64 = 0u64.wrapping_sub(choice as u64);
    unsafe {
      let mask = _mm512_set1_epi64(mask_val as i64);
      let src_vec = _mm512_loadu_si512(other as *const Self as *const __m512i);
      let dest_vec = _mm512_loadu_si512(self as *const Self as *const __m512i);
      let blended =
        _mm512_or_si512(_mm512_and_si512(mask, src_vec), _mm512_andnot_si512(mask, dest_vec));
      _mm512_storeu_si512(self as *mut Self as *mut __m512i, blended);
    }
  }
}

#[cfg(target_feature = "avx2")]
impl _Cmovbase for [u8; 32] {
  #[inline]
  fn cmov_base(&mut self, other: &Self, choice: bool) {
    let mask_val: u64 = 0u64.wrapping_sub(choice as u64);
    unsafe {
      let mask = _mm256_set1_epi64x(mask_val as i64);
      let src_vec = _mm256_loadu_si256(other as *const Self as *const __m256i);
      let dest_vec = _mm256_loadu_si256(self as *const Self as *const __m256i);
      let blended =
        _mm256_or_si256(_mm256_and_si256(mask, src_vec), _mm256_andnot_si256(mask, dest_vec));
      _mm256_storeu_si256(self as *mut Self as *mut __m256i, blended);
    }
  }
}

#[cfg(target_feature = "sse2")]
impl _Cmovbase for u128 {
  #[inline]
  fn cmov_base(&mut self, other: &Self, choice: bool) {
    let mask_val = 0u64.wrapping_sub(choice as u64);
    unsafe {
      let mask = _mm_set1_epi64x(mask_val as i64);
      let src_vec = _mm_loadu_si128(other as *const Self as *const __m128i);
      let dest_vec = _mm_loadu_si128(self as *const Self as *const __m128i);
      let blended = _mm_or_si128(_mm_and_si128(mask, src_vec), _mm_andnot_si128(mask, dest_vec));
      _mm_storeu_si128(self as *mut Self as *mut __m128i, blended);
    }
  }
}

// This is leading to slower code:
// impl _Cmovbase for u64 {
//   #[inline]
//   fn cmov_base(&mut self, other: &Self, choice: bool) {
//     let self_ptr = self as *mut Self;
//     let other_ptr = other as *const Self;

//     // let mut tmp: u64 = unsafe { MaybeUninit::uninit().assume_init() } ;
//     let mut tmp = MaybeUninit::<u64>::uninit();
//     unsafe {
//       asm!(
//         "mov {tmp}, [{i1}]",
//         "test {mcond}, {mcond}",
//         "cmovnz {tmp}, [{i2}]",
//         "mov [{i1}], {tmp}",
//         tmp = inout(reg) tmp,
//         i1 = in(reg) self_ptr,
//         i2 = in(reg) other_ptr,
//         mcond = in(reg) choice as u64,
//         options(nostack)
//       );
//     }
//   }
// }

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
