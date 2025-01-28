//! Commonly used base types and primitives for the oblivious algorithms.

pub mod asm;

pub mod traits;

pub mod indexable;

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
pub mod asm_x86;

#[cfg(target_arch = "aarch64")]
pub mod asm_aarch64;