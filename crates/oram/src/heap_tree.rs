//! Represents a heap tree as an array and provides functions to access it.
//!

/// Represents a heap tree structure.
#[derive(Debug)]
pub struct HeapTree<T> {
  tree: Vec<T>,  // Actual storage container
  height: usize, // Height of the tree, public
}

impl<T> HeapTree<T>
where
  T: Default + Clone,
{
  /// Initialized a new heap tree with a certain height
  pub fn new(height: usize) -> Self {
    let tree = vec![T::default(); 2usize.pow(1 + height as u32) - 1];
    Self { tree, height }
  }
}

impl<T> HeapTree<T>
where
  T: Clone,
{
  /// Initialized a new heap tree with a certain height and a default value
  pub fn new_with(height: usize, default: T) -> Self {
    let tree = vec![default; 2usize.pow(1 + height as u32) - 1];
    Self { tree, height }
  }
}

impl<T> HeapTree<T> {
  /// Get a node of a certain path at a certain depth
  /// Reveals depth and path
  pub fn get_path_at_depth(&self, depth: usize, path: usize) -> &T {
    debug_assert!(depth < self.height);
    let level_offset = 2usize.pow(depth as u32) - 1;
    let index = level_offset + (path >> (self.height - depth));

    // UNDONE(): Make sure this doesn't have bounds checking and is safe
    &self.tree[index]
  }

  /// Get a node of a certain path at a certain depth
  /// /// Reveals depth and path
  pub fn get_path_at_depth_mut(&mut self, depth: usize, path: usize) -> &mut T {
    debug_assert!(depth < self.height);
    let level_offset = 2usize.pow(depth as u32) - 1;
    let index = level_offset + (path >> (self.height - depth));

    // UNDONE(): Make sure this doesn't have bounds checking and is safe
    &mut self.tree[index]
  }
}

// UNDONE(): write tests for this module
