//! Represents a heap tree as an array and provides functions to access it.
//!
//! The tree is represented by a reverse lexicographical order binary tree.
//!                            0
//!                     1             2
//!                3         5    4       6
//! Path:          0         2    1       3

use crate::prelude::PositionType;

/// Represents a heap tree structure.
#[derive(Debug)]
pub struct HeapTree<T> {
  /// Actual storage container
  pub(crate) tree: Vec<T>,
  /// Height of the tree, public (tree with a single element has height 1)
  pub height: usize,
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
  /// Get the index of a node at a certain depth and path
  #[inline]
  pub fn get_index(&self, depth: usize, path: PositionType) -> usize {
    debug_assert!(depth < self.height);
    let level_offset = (1 << depth) - 1;
    let mask = level_offset as PositionType;
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

  /// Given a path and a node at certain depth, return the other child of that node's parent.
  pub fn get_sibling(&self, depth: usize, path: PositionType) -> &T {
    let new_path = path ^ (1 << (depth - 1));
    self.get_path_at_depth(depth, new_path)
  }
}

#[cfg(test)]
mod tests {
  use crate::prelude::PositionType;

  fn print_depth_pos_index(height: usize, depth: usize, path: PositionType) {
    debug_assert!(depth < height);
    let level_offset = (1 << depth) - 1;
    let mask = level_offset as PositionType;
    let _ret = level_offset + (path & mask) as usize;
  }
  #[test]
  fn print_heap_tree_info() {
    for depth in 0..3 {
      for path in 0..4 {
        print_depth_pos_index(3, depth, path);
      }
    }
  }
}
