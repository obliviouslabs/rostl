//! Traits for indexable types.
//
use std::ops::{Index, IndexMut};

/// Anything that has length.
pub trait Length {
  /// Returns the length of the indexable.
  fn len(&self) -> usize;
}
impl<T> Length for [T] {
  fn len(&self) -> usize {
    <[T]>::len(self)
  }
}
impl<T> Length for &mut [T] {
  fn len(&self) -> usize {
    <[T]>::len(self)
  }
}
impl<T> Length for Vec<T> {
  fn len(&self) -> usize {
    Self::len(self)
  }
}

/// An Indexable that can be used in the algorithms in this library
/// An indexable trait. `IndexMux` should modify in place, `length` should be consistent with the indexable.
pub trait Indexable<T>: Index<usize, Output = T> + IndexMut<usize, Output = T> + Length {}

impl<T, C> Indexable<T> for C where
  C: Index<usize, Output = T> + IndexMut<usize, Output = T> + Length + ?Sized
{
}
