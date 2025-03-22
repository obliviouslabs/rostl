//! Represents a heap tree as an array and provides functions to access it.
//!

use crate::prelude::PositionType;

/// Represents a heap tree structure.
#[derive(Debug)]
pub struct HeapTree<T> {
  tree: Vec<T>,  // Actual storage container
  height: usize, // Height of the tree, public, tree with a single element has height 1
}

impl<T> HeapTree<T>
where
  T: Default + Clone,
{
  /// Initialized a new heap tree with a certain height
  pub fn new(height: usize) -> Self {
    let tree = vec![T::default(); 2usize.pow(height as u32) - 1];
    Self { tree, height }
  }
}

impl<T> HeapTree<T>
where
  T: Clone,
{
  /// Initialized a new heap tree with a certain height and a default value
  pub fn new_with(height: usize, default: T) -> Self {
    let tree = vec![default; 2usize.pow(height as u32) - 1];
    Self { tree, height }
  }
}

impl<T> HeapTree<T> {
  #[inline]
  fn get_index(&self, depth: usize, path: PositionType) -> usize {
    debug_assert!(depth < self.height);
    let level_offset = 2usize.pow(depth as u32) - 1;
    let mask = (1 << depth) - 1;
    level_offset + (path & mask) as usize
  }

  /// Get a node of a certain path at a certain depth
  /// Reveals depth and path
  #[inline]
  pub fn get_path_at_depth(&self, depth: usize, path: PositionType) -> &T {
    let index = self.get_index(depth, path);

    // UNDONE(git-10): Make sure this doesn't have bounds checking and is safe
    &self.tree[index]
  }

  /// Get a node of a certain path at a certain depth
  /// Reveals depth and path
  #[inline]
  pub fn get_path_at_depth_mut(&mut self, depth: usize, path: PositionType) -> &mut T {
    let index = self.get_index(depth, path);

    // UNDONE(git-10): Make sure this doesn't have bounds checking and is safe
    &mut self.tree[index]
  }
}
