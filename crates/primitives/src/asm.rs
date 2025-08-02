//! Assembly implementations of the `Cmov` trait.
//!
use crate::traits::{Cmov, _Cmovbase};

// UNDONE(git-20): Once rust generics support either specialization, negative trait bounds, or finalizations this can be turned into a generic.
// Until then, we have this ugly macro.
/// The shared body for cmov.
/// Any file that uses the macro should include be in a module that includes the bytemuck crate and should also include:
/// ```ignore
/// use rostl_primitives::{impl_cmov_for_pod, cmov_body, cxchg_body, traits::_Cmovbase};
/// ```
///
#[macro_export]
macro_rules! cmov_body {
  ($self:ident, $other:ident, $choice:ident) => {{
    let self_bytes = bytemuck::bytes_of_mut($self);
    let other_bytes = bytemuck::bytes_of($other);

    let mut i = 0;

    #[cfg(all(target_feature = "avx512f", target_feature = "avx512vl"))]
    {
      // Process in chunks of 64 bytes ([u8; 64])
      while i + 64 <= self_bytes.len() {
        let self_chunk_1 = &mut self_bytes[i..i + 64];
        let self_chunk: &mut [u8; 64] = unsafe { &mut *(self_chunk_1.as_mut_ptr() as *mut [u8; 64]) };
        let other_chunk_1 = &other_bytes[i..i + 64];
        let other_chunk: &[u8; 64] = unsafe { &*(other_chunk_1.as_ptr() as *const [u8; 64]) };

        self_chunk.cmov_base(&other_chunk, $choice);

        i += 64;
      }
    }

    // // Process in chunks of 32 bytes ([u8; 32])
    #[cfg(target_feature = "avx2")]
    {
      while i + 32 <= self_bytes.len() {
        let self_chunk_1 = &mut self_bytes[i..i + 32];
        let self_chunk: &mut [u8; 32] = unsafe { &mut *(self_chunk_1.as_mut_ptr() as *mut [u8; 32]) };
        let other_chunk_1 = &other_bytes[i..i + 32];
        let other_chunk: &[u8; 32] = unsafe { &*(other_chunk_1.as_ptr() as *const [u8; 32]) };

        self_chunk.cmov_base(&other_chunk, $choice);

        i += 32;
      }
    }

    // This doesn't check for alignemnt, so it might fail or be slower...
    #[cfg(target_feature = "sse2")]
    {
      while i + 16 <= self_bytes.len() {
        let self_chunk = &mut self_bytes[i..i + 16];
        let other_chunk = &other_bytes[i..i + 16];
        let self_u64 = u128::from_ne_bytes(self_chunk.try_into().unwrap());
        let other_u64 = u128::from_ne_bytes(other_chunk.try_into().unwrap());
        let mut result = self_u64;
        result.cmov_base(&other_u64, $choice);
        self_chunk.copy_from_slice(&result.to_ne_bytes());
        i += 16;
      }
    }

    // This checks for alignement but it seems it isn't needed for correctness in recent cpus? (they only require 8 byte alignment??)
    // #[cfg(target_feature = "sse2")]
    // {
    //   // Process in chunks of 16 bytes (u128)
    //   while i + 16 <= self_bytes.len() {
    //     let self_chunk = &mut self_bytes[i..i + 16];
    //     let other_chunk = &other_bytes[i..i + 16];

    //     debug_assert_eq!(self_chunk.as_ptr() as usize % std::mem::align_of::<u128>(), 0, "Not aligned");
    //     debug_assert_eq!(other_chunk.as_ptr() as usize % std::mem::align_of::<u128>(), 0, "Not aligned");

    //     let self_u128 = unsafe { &mut *(self_chunk.as_mut_ptr() as *mut u128) };
    //     let other_u128 = unsafe { & *(other_chunk.as_ptr() as *const u128) };

    //     self_u128.cmov_base(&other_u128, $choice);
    //     i += 16;
    //   }
    // }


    // Process in chunks of 8 bytes (u64)
    while i + 8 <= self_bytes.len() {
      let self_chunk = &mut self_bytes[i..i + 8];
      let other_chunk = &other_bytes[i..i + 8];

      debug_assert_eq!(self_chunk.as_ptr() as usize % std::mem::align_of::<u64>(), 0, "Not aligned");
      debug_assert_eq!(other_chunk.as_ptr() as usize % std::mem::align_of::<u64>(), 0, "Not aligned");

      let self_u64 = unsafe { &mut *(self_chunk.as_mut_ptr() as *mut u64) };
      let other_u64 = unsafe { & *(other_chunk.as_ptr() as *const u64) };

      self_u64.cmov_base(&other_u64, $choice);
      i += 8;
    }

    // Process in chunnks of 4 bytes (u32)
    while i + 4 <= self_bytes.len() {
      let self_chunk = &mut self_bytes[i..i + 4];
      let other_chunk = &other_bytes[i..i + 4];

      debug_assert_eq!(self_chunk.as_ptr() as usize % std::mem::align_of::<u32>(), 0, "Not aligned");
      debug_assert_eq!(other_chunk.as_ptr() as usize % std::mem::align_of::<u32>(), 0, "Not aligned");

      let self_u32 = unsafe { &mut *(self_chunk.as_mut_ptr() as *mut u32) };
      let other_u32 = unsafe { & *(other_chunk.as_ptr() as *const u32) };

      self_u32.cmov_base(&other_u32, $choice);
      i += 4;
    }

    while i + 2 <= self_bytes.len() {
      let self_chunk = &mut self_bytes[i..i + 2];
      let other_chunk = &other_bytes[i..i + 2];

      debug_assert_eq!(self_chunk.as_ptr() as usize % std::mem::align_of::<u16>(), 0, "Not aligned");
      debug_assert_eq!(other_chunk.as_ptr() as usize % std::mem::align_of::<u16>(), 0, "Not aligned");

      let self_u16 = unsafe { &mut *(self_chunk.as_mut_ptr() as *mut u16) };
      let other_u16 = unsafe { & *(other_chunk.as_ptr() as *const u16) };

      self_u16.cmov_base(&other_u16, $choice);
      i += 2;
    }

    // Process remaining u8.
    if i < self_bytes.len() {
      let self_u8 = &mut self_bytes[i];
      let other_u8 = other_bytes[i];
      self_u8.cmov_base(&other_u8, $choice);
    }
  }};
}

/// UNDONE(git-20)
/// Optimized cxchg
#[macro_export]
macro_rules! cxchg_body {
  ($self:ident, $other:ident, $choice:ident) => {{
    let self_bytes = bytemuck::bytes_of_mut($self);
    let other_bytes = bytemuck::bytes_of_mut($other);

    let mut i = 0;

    // #[cfg(all(target_feature = "avx512f", target_feature = "avx512vl"))]
    // {
    //   // Process in chunks of 64 bytes ([u8; 64])
    //   while i + 64 <= self_bytes.len() {
    //     let self_chunk_1 = &mut self_bytes[i..i + 64];
    //     let self_chunk: &mut [u8; 64] = unsafe { &mut *(self_chunk_1.as_mut_ptr() as *mut [u8; 64]) };
    //     let other_chunk_1 = &other_bytes[i..i + 64];
    //     let other_chunk: &mut [u8; 64] = unsafe { &mut *(other_chunk_1.as_mut_ptr() as *mut [u8; 64]) };

    //     self_chunk.cxchg_base(other_chunk, $choice);

    //     i += 64;
    //   }
    // }

    // // Process in chunks of 32 bytes ([u8; 32])
    #[cfg(target_feature = "avx2")]
    {
      while i + 32 <= self_bytes.len() {
        let self_chunk_1 = &mut self_bytes[i..i + 32];
        let self_chunk: &mut [u8; 32] = unsafe { &mut *(self_chunk_1.as_mut_ptr() as *mut [u8; 32]) };
        let other_chunk_1 = &mut other_bytes[i..i + 32];
        let other_chunk: &mut [u8; 32] = unsafe { &mut *(other_chunk_1.as_mut_ptr() as *mut [u8; 32]) };

        self_chunk.cxchg_base(other_chunk, $choice);

        i += 32;
      }
    }


    // UNDONE(): This seems to be broken in rust 1.89:
    // #[cfg(target_feature = "sse2")]
    // {
    //   // Process in chunks of 16 bytes (u128)
    //   while i + 16 <= self_bytes.len() {
    //     let self_chunk = &mut self_bytes[i..i + 16];
    //     let other_chunk = &mut other_bytes[i..i + 16];
    //     let self_u128 = unsafe { &mut *(self_chunk.as_mut_ptr() as *mut u128) };
    //     let other_u128 = unsafe { &mut *(other_chunk.as_mut_ptr() as *mut u128) };

    //     self_u128.cxchg_base(other_u128, $choice);
    //     i += 16;
    //   }
    // }


    // Process in chunks of 8 bytes (u64)
    while i + 8 <= self_bytes.len() {
      let self_chunk = &mut self_bytes[i..i + 8];
      let other_chunk = &mut other_bytes[i..i + 8];

      debug_assert_eq!(self_chunk.as_ptr() as usize % std::mem::align_of::<u64>(), 0, "Not aligned");
      debug_assert_eq!(other_chunk.as_ptr() as usize % std::mem::align_of::<u64>(), 0, "Not aligned");

      let self_u64 = unsafe { &mut *(self_chunk.as_mut_ptr() as *mut u64) };
      let other_u64 = unsafe { &mut *(other_chunk.as_mut_ptr() as *mut u64) };

      self_u64.cxchg_base(other_u64, $choice);
      i += 8;
    }

    // Process in chunnks of 4 bytes (u32)
    while i + 4 <= self_bytes.len() {
      let self_chunk = &mut self_bytes[i..i + 4];
      let other_chunk = &mut other_bytes[i..i + 4];

      debug_assert_eq!(self_chunk.as_ptr() as usize % std::mem::align_of::<u32>(), 0, "Not aligned");
      debug_assert_eq!(other_chunk.as_ptr() as usize % std::mem::align_of::<u32>(), 0, "Not aligned");

      let self_u32 = unsafe { &mut *(self_chunk.as_mut_ptr() as *mut u32) };
      let other_u32 = unsafe { &mut *(other_chunk.as_mut_ptr() as *mut u32) };

      self_u32.cxchg_base(other_u32, $choice);
      i += 4;
    }

    while i + 2 <= self_bytes.len() {
      let self_chunk = &mut self_bytes[i..i + 2];
      let other_chunk = &mut other_bytes[i..i + 2];

      debug_assert_eq!(self_chunk.as_ptr() as usize % std::mem::align_of::<u16>(), 0, "Not aligned");
      debug_assert_eq!(other_chunk.as_ptr() as usize % std::mem::align_of::<u16>(), 0, "Not aligned");

      let self_u16 = unsafe { &mut *(self_chunk.as_mut_ptr() as *mut u16) };
      let other_u16 = unsafe { &mut *(other_chunk.as_mut_ptr() as *mut u16) };

      self_u16.cxchg_base(other_u16, $choice);
      i += 2;
    }

    // Process remaining u8.
    if i < self_bytes.len() {
      let self_u8 = &mut self_bytes[i];
      let other_u8 = &mut other_bytes[i];
      self_u8.cxchg_base(other_u8, $choice);
    }
  }};
}

/// Automatically implement the Cmov trait for a concrete type that is a pod.
///
#[macro_export]
macro_rules! impl_cmov_for_pod {
  // Concrete type case.
  ($ty:ty) => {
    impl Cmov for $ty
    where
      $ty: bytemuck::Pod,
    {
      #[inline]
      fn cmov(&mut self, other: &Self, choice: bool) {
        cmov_body!(self, other, choice);
      }
      #[inline]
      fn cxchg(&mut self, other: &mut Self, choice: bool) {
        cxchg_body!(self, other, choice);
      }
    }
  };
}

/// Automatically implement the Cmov trait for a generic type that is a pod.
/// usage:
/// ```ignore
/// impl_cmov_for_generic_pod!(Type<A1, A2...> [; where A1: ..., A2: ...])
/// ```
///
#[macro_export]
macro_rules! impl_cmov_for_generic_pod {
    // Case with no extra where clause.
    ($t:ident < $($gen:ident),* >) => {
        impl<$($gen),*> Cmov for $t<$($gen),*>
        where
            $t<$($gen),*>: bytemuck::Pod,
        {
            #[inline]
            fn cmov(&mut self, other: &Self, choice: bool) {
                cmov_body!(self, other, choice);
            }
            #[inline]
            fn cxchg(&mut self, other: &mut Self, choice: bool) {
              cxchg_body!(self, other, choice);
            }
        }
    };
    // Case with an extra where clause.
    ($t:ident < $($gen:ident),* >; where $($wc:tt)+) => {
        impl<$($gen),*> Cmov for $t<$($gen),*>
        where
            $t<$($gen),*>: bytemuck::Pod,
            $($wc)+
        {
            #[inline]
            fn cmov(&mut self, other: &Self, choice: bool) {
                cmov_body!(self, other, choice);
            }
            #[inline]
            fn cxchg(&mut self, other: &mut Self, choice: bool) {
              cxchg_body!(self, other, choice);
            }
        }
    };
}

impl_cmov_for_pod!(usize);
impl_cmov_for_pod!(u128);
impl_cmov_for_pod!(u64);
impl_cmov_for_pod!(u32);
impl_cmov_for_pod!(u16);
impl_cmov_for_pod!(u8);
impl_cmov_for_pod!(i128);
impl_cmov_for_pod!(i64);
impl_cmov_for_pod!(i32);
impl_cmov_for_pod!(i16);
impl_cmov_for_pod!(i8);

impl Cmov for bool {
  #[inline]
  fn cmov(&mut self, other: &Self, choice: bool) {
    if choice {
      *self = *other;
    }
  }

  #[inline]
  fn cxchg(&mut self, other: &mut Self, choice: bool) {
    let c = *self;
    self.cmov(other, choice);
    other.cmov(&c, choice);
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
