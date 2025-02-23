//! Assembly implementations of the `Cmov` trait.
//!
use crate::traits::{Cmov, _Cmovbase};

// UNDONE(): Once rust generics support either specialization, negative trait bounds, or finalizations this can be turned into a generic.
// Until then, we have this ugly macro.
/// The shared body for cmov.
/// Any file that uses the macro should include be in a module that includes the bytemuck crate and should also include:
/// ```ignore
/// use rods_primitives::{impl_cmov_for_pod, cmov_body, traits::_Cmovbase};
/// ```
///
#[macro_export]
macro_rules! cmov_body {
  ($self:ident, $other:ident, $choice:ident) => {{
    let self_bytes = bytemuck::bytes_of_mut($self);
    let other_bytes = bytemuck::bytes_of($other);
    let mut i = 0;

    // Process in chunks of 8 bytes (u64)
    while i + 8 <= self_bytes.len() {
      let self_chunk = &mut self_bytes[i..i + 8];
      let other_chunk = &other_bytes[i..i + 8];

      let self_u64 = u64::from_ne_bytes(self_chunk.try_into().unwrap());
      let other_u64 = u64::from_ne_bytes(other_chunk.try_into().unwrap());

      let mut result = self_u64;
      result.cmov_base(&other_u64, $choice);

      self_chunk.copy_from_slice(&result.to_ne_bytes());
      i += 8;
    }

    // Process in chunks of 4 bytes (u32)
    while i + 4 <= self_bytes.len() {
      let self_chunk = &mut self_bytes[i..i + 4];
      let other_chunk = &other_bytes[i..i + 4];

      let self_u32 = u32::from_ne_bytes(self_chunk.try_into().unwrap());
      let other_u32 = u32::from_ne_bytes(other_chunk.try_into().unwrap());

      let mut result = self_u32;
      result.cmov_base(&other_u32, $choice);

      self_chunk.copy_from_slice(&result.to_ne_bytes());
      i += 4;
    }

    // Process in chunks of 2 bytes (u16)
    while i + 2 <= self_bytes.len() {
      let self_chunk = &mut self_bytes[i..i + 2];
      let other_chunk = &other_bytes[i..i + 2];

      let self_u16 = u16::from_ne_bytes(self_chunk.try_into().unwrap());
      let other_u16 = u16::from_ne_bytes(other_chunk.try_into().unwrap());

      let mut result = self_u16;
      result.cmov_base(&other_u16, $choice);

      self_chunk.copy_from_slice(&result.to_ne_bytes());
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
        }
    };
}

impl_cmov_for_pod!(usize);
impl_cmov_for_pod!(u64);
impl_cmov_for_pod!(u32);
impl_cmov_for_pod!(u16);
impl_cmov_for_pod!(u8);
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
}

impl<A: Cmov, B: Cmov> Cmov for (A, B) {
  #[inline]
  fn cmov(&mut self, other: &Self, choice: bool) {
    self.0.cmov(&other.0, choice);
    self.1.cmov(&other.1, choice);
  }
}

impl<A: Cmov, B: Cmov, C: Cmov> Cmov for (A, B, C) {
  #[inline]
  fn cmov(&mut self, other: &Self, choice: bool) {
    self.0.cmov(&other.0, choice);
    self.1.cmov(&other.1, choice);
    self.2.cmov(&other.2, choice);
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
