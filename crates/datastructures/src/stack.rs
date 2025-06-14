//! This module implements an oblivious stack
// The stack is implemented as a linked list on top of NRORAM.

use bytemuck::{Pod, Zeroable};
use rand::{rngs::ThreadRng, Rng};
use rostl_oram::{
  circuit_oram::CircuitORAM,
  prelude::{PositionType, DUMMY_POS},
};
use rostl_primitives::{
  cmov_body, cxchg_body, impl_cmov_for_generic_pod, indexable::Length, traits::Cmov,
  traits::_Cmovbase,
};

#[repr(align(8))]
#[derive(Debug, Default, Clone, Copy, Zeroable)]
struct StackElement<T>
where
  T: Cmov + Pod,
{
  value: T,
  next: PositionType,
}
unsafe impl<T: Cmov + Pod> Pod for StackElement<T> {}
impl_cmov_for_generic_pod!(StackElement<T>; where T: Cmov + Pod);

/// Implements a stack with a fixed maximum size.
/// The stack access pattern and size are oblivious.
/// The stack is implemented as a linked list on top of NRORAM.
/// # Invariants
/// * 1) The linked list of elements is in monotonic decreasing order of Ids.
#[derive(Debug)]
pub struct Stack<T>
where
  T: Cmov + Pod,
{
  oram: CircuitORAM<StackElement<T>>,
  top: PositionType,
  size: usize,
  rng: ThreadRng,
}

impl<T> Stack<T>
where
  T: Cmov + Pod + Default + Clone + std::fmt::Debug,
{
  /// Creates a new stack.
  pub fn new(max_size: usize) -> Self {
    Self { oram: CircuitORAM::new(max_size), top: DUMMY_POS, size: 0, rng: rand::rng() }
  }

  /// Pushes a new element on the stack if `real` is true.
  /// If `real` is false, the element is not pushed and the stack size is not incremented.
  pub fn maybe_push(&mut self, real: bool, value: T) {
    debug_assert!(!real || self.size < self.oram.max_n);

    let new_id = self.size + 1; // inv1
    let read_pos = self.rng.random_range(0..self.oram.max_n as PositionType);

    let mut new_pos = self.rng.random_range(0..self.oram.max_n as PositionType);
    new_pos.cmov(&DUMMY_POS, !real); // if not real, new_pos is DUMMY_POS, oram will ignore the write

    let wv = StackElement { value, next: self.top };

    let _found = self.oram.write_or_insert(read_pos, new_pos, new_id, wv);
    debug_assert!(!_found);

    self.top.cmov(&new_pos, real); // if real, top is new_pos
    self.size.cmov(&(self.size + 1), real);
  }

  /// Pops the top element from the stack if `real` is true.
  /// The popped element is returned in `out`.
  /// The stack size is decremented by 1 if `real` is true.
  /// If `real` is false, the element is not popped, the stack size is not decremented, and `out` is not modified.
  pub fn maybe_pop(&mut self, real: bool, out: &mut T) {
    debug_assert!(!real || self.size > 0);

    let target_id = self.size; // inv1 - the position of the top of the stack is the size of the stack.
    let mut read_pos = self.rng.random_range(0..self.oram.max_n as PositionType);
    read_pos.cmov(&self.top, real);
    let mut new_pos = read_pos;
    new_pos.cmov(&DUMMY_POS, real); // if real, we should delete the top element. if not real, we should not change the read element.

    let mut imse = StackElement::default();

    self.oram.read(read_pos, read_pos, target_id, &mut imse);

    out.cmov(&imse.value, real);
    self.top.cmov(&imse.next, real);
    self.size.cmov(&(self.size - 1), real);
  }
}

impl<T> Length for Stack<T>
where
  T: Cmov + Pod,
{
  fn len(&self) -> usize {
    self.size
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_stack() {
    let mut stack = Stack::<u32>::new(10);
    let mut out = 0;
    stack.maybe_push(true, 100);
    assert_eq!(stack.len(), 1);
    stack.maybe_push(true, 222);
    assert_eq!(stack.len(), 2);
    stack.maybe_push(true, 3333);
    assert_eq!(stack.len(), 3);
    stack.maybe_push(false, 123214);
    assert_eq!(stack.len(), 3);
    stack.maybe_pop(true, &mut out);
    assert_eq!(stack.len(), 2);
    assert_eq!(out, 3333);
    stack.maybe_pop(true, &mut out);
    assert_eq!(stack.len(), 1);
    assert_eq!(out, 222);
    out = 123;
    stack.maybe_pop(false, &mut out);
    assert_eq!(stack.len(), 1);
    assert_eq!(out, 123);
    stack.maybe_pop(true, &mut out);
    assert_eq!(stack.len(), 0);
    assert_eq!(out, 100);
  }

  // UNDONE(git-61): Benchmark Stack.
}
