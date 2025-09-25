#![cfg(any(target_arch = "x86", target_arch = "x86_64"))]

//! x86 assembly implementations of the `Cmov` trait.
//!
use core::arch::x86_64::*;
use std::arch::asm;

use crate::traits::{Cmov, _Cmovbase};

// CMOV -------------------------
//

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
  #[inline]
  fn cxchg_base(&mut self, other: &mut Self, choice: bool) {
    // Compute blend_mask in branchless fashion:
    // In the reference: blend_mask = (__mmask8)(!cond) - 1;
    // When choice is true: !choice is false (0), 0 - 1 yields 0xFF.
    // When choice is false: !choice is true (1), 1 - 1 yields 0.
    let lane_mask: i32 = -(choice as i32);
    unsafe {
      // Create a 512-bit vector with each 32-bit lane filled with mask.
      let mask_vec = _mm512_set1_epi32(lane_mask);
      // Load the full 64 bytes from each operand.
      let mut vec1 = _mm512_loadu_si512(self.as_ptr() as *const __m512i);
      let mut vec2 = _mm512_loadu_si512(other.as_ptr() as *const __m512i);
      // Compute the difference via XOR.
      let diff = _mm512_xor_si512(vec1, vec2);
      // Mask the difference so that if choice is false, diff_masked is zero.
      let diff_masked = _mm512_and_si512(diff, mask_vec);
      // Apply the masked difference to swap the values.
      vec1 = _mm512_xor_si512(vec1, diff_masked);
      vec2 = _mm512_xor_si512(vec2, diff_masked);
      _mm512_storeu_si512(self.as_mut_ptr() as *mut __m512i, vec1);
      _mm512_storeu_si512(other.as_mut_ptr() as *mut __m512i, vec2);
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

  #[inline]
  fn cxchg_base(&mut self, other: &mut Self, choice: bool) {
    unsafe {
      let mask = _mm256_set1_epi32(0i32.wrapping_sub(choice as i32));
      let vec1 = _mm256_loadu_si256(self.as_ptr() as *const __m256i);
      let vec2 = _mm256_loadu_si256(other.as_ptr() as *const __m256i);
      let diff = _mm256_xor_si256(vec1, vec2);
      let diff_masked = _mm256_and_si256(diff, mask);
      let new_vec1 = _mm256_xor_si256(vec1, diff_masked);
      let new_vec2 = _mm256_xor_si256(vec2, diff_masked);
      _mm256_storeu_si256(self.as_mut_ptr() as *mut __m256i, new_vec1);
      _mm256_storeu_si256(other.as_mut_ptr() as *mut __m256i, new_vec2);
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
  #[inline]
  fn cxchg_base(&mut self, other: &mut Self, choice: bool) {
    unsafe {
      let mask = _mm_set1_epi16(0i16.wrapping_sub(choice as i16));
      let vec1 = _mm_loadu_si128(self as *const Self as *const __m128i);
      let vec2 = _mm_loadu_si128(other as *const Self as *const __m128i);
      let diff = _mm_xor_si128(vec1, vec2);
      let diff_masked = _mm_and_si128(diff, mask);
      let new_vec1 = _mm_xor_si128(vec1, diff_masked);
      let new_vec2 = _mm_xor_si128(vec2, diff_masked);
      _mm_storeu_si128(self as *mut Self as *mut __m128i, new_vec1);
      _mm_storeu_si128(other as *mut Self as *mut __m128i, new_vec2);
    }
  }
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
    let mut su32 = *self as u32;
    let ou32 = *other as u32;
    su32.cmov(&ou32, choice);
    *self = su32 as Self;
  }

  #[inline]
  fn cxchg_base(&mut self, other: &mut Self, choice: bool) {
    let c = *self;
    self.cmov_base(other, choice);
    other.cmov_base(&c, choice);
  }
}
