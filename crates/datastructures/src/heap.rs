//! Implements [path oblivious heap](https://eprint.iacr.org/2019/274).
use bytemuck::{Pod, Zeroable};
use rand::{rngs::ThreadRng, Rng};
use rods_oram::{
  circuit_oram::{remove_element, write_block_to_empty_slot, Block, CircuitORAM, S, Z},
  heap_tree::HeapTree,
  prelude::{PositionType, K},
};
use rods_primitives::traits::{Cmov, _Cmovbase};
use rods_primitives::{cmov_body, cxchg_body, impl_cmov_for_generic_pod};

#[derive(Clone, Copy, Debug, Zeroable)]
#[repr(C)]
/// A logical heap element.
pub struct HeapElement<V>
where
  V: Cmov + Pod,
{
  /// The key associated with the heap element.
  pub key: K,
  /// The value associated with the heap element.
  pub value: V,
}
unsafe impl<V: Cmov + Pod> Pod for HeapElement<V> {}
impl_cmov_for_generic_pod!(HeapElement<V>; where V: Cmov + Pod);
impl<V: Cmov + Pod> Default for HeapElement<V> {
  fn default() -> Self {
    Self { key: K::MAX, value: V::zeroed() }
  }
}

#[derive(Debug)]
/// An oblivious heap.
/// Elements are stored in an ORAM, along with information about the location of the minimum element in each subtree.
/// # Invariants
/// * `metadata` stores the minimum element in each subtree.
/// * Heap elements are stored in a non-recursive ORAM.
/// * After insertion, the oram key (timestamp) and path for an element do not change.
pub struct Heap<V>
where
  V: Cmov + Pod,
{
  /// The heap elements.
  pub data: CircuitORAM<HeapElement<V>>,
  /// The metadata tree used for storing the element with minimum key in the subtree.
  pub metadata: HeapTree<Block<HeapElement<V>>>,
  /// Thread local rng.
  pub rng: ThreadRng,
  /// maximum size of the heap.
  pub max_size: usize,
  /// timestamp: usize,
  pub timestamp: K,
}

impl<V> Heap<V>
where
  V: Cmov + Pod + std::fmt::Debug,
{
  /// Creates a Heap with maximum size of `n` elements.
  pub fn new(n: usize) -> Self {
    let data = CircuitORAM::new(n);
    let default_value = Block::<HeapElement<V>>::default();
    let metadata = HeapTree::new_with(data.h, default_value);
    Self { data, metadata, rng: rand::rng(), max_size: n, timestamp: 0 }
  }

  /// Finds the minimum element in the heap.
  /// # Returns
  /// * The minimum element in the heap. => if the heap is non-empty.
  /// * A `HeapElement<V>` with `pos = DUMMY` => if the heap is empty.
  pub fn find_min(&self) -> Block<HeapElement<V>> {
    let mut min_node = *self.metadata.get_path_at_depth(0, 0);

    for elem in &self.data.stash[0..S] {
      let should_mov = (!elem.is_empty()) & (elem.value.key < min_node.value.key);
      min_node.cmov(elem, should_mov);
    }

    min_node
  }

  fn evict(&mut self, pos: PositionType) {
    self.data.read_path_and_get_nodes(pos);
    self.data.evict_once_fast(pos);
    self.data.write_back_path(pos);
  }

  /// Prints the heap for debugging purposes.
  #[cfg(test)]
  pub fn print_for_debug(&self) {
    let data = &self.data;
    println!("Stash: {:?}", data.stash);
    for i in 0..data.h {
      print!("Level {}: ", i);
      for j in 0..(1 << i) {
        print!("{} ", j << (data.h - 1 - i));
        print!("data.h:{} ", data.h);
        print!(
          "{:?} ",
          data.tree.get_path_at_depth(
            i,
            ((j << (data.h - 1 - i)) as u32).reverse_bits() >> (32 - data.h + 1)
          )
        );
      }
      println!();
    }
  }

  // Updates the metadata for the minimum element along a path `pos`.
  // # Preconditions:
  // * The path is already loaded into the stash.
  // * All the metadata except for this path is correct.
  fn update_min(&mut self, pos: PositionType) {
    let data = &self.data;
    let mut h_index = self.metadata.height;
    let metadata = &mut self.metadata;

    let mut curr_min = Block::<HeapElement<V>>::default();
    curr_min.value.key = K::MAX;

    for elems in data.stash[S..(S + self.data.h * Z)].chunks(2).rev() {
      for elem in elems {
        let should_mov = (!elem.is_empty()) & (elem.value.key < curr_min.value.key);
        curr_min.cmov(elem, should_mov);
      }

      if h_index != metadata.height {
        let sibling = metadata.get_sibling(h_index, pos);

        let should_mov = (!sibling.is_empty()) & (sibling.value.key < curr_min.value.key);
        curr_min.cmov(sibling, should_mov);
      }

      *metadata.get_path_at_depth_mut(h_index - 1, pos) = curr_min;

      h_index -= 1;
    }
  }

  /// Inserts a new element `value` with priority `key` into the heap.
  /// # Returns
  /// * the position and timestamp of the inserted element.
  pub fn insert(&mut self, key: K, value: V) -> (PositionType, K) {
    let new_pos = self.rng.random_range(0..self.data.max_n as PositionType);
    let oram_key: K = self.timestamp;
    self.timestamp += 1;
    let heap_value = HeapElement::<V> { key, value };

    write_block_to_empty_slot(
      &mut self.data.stash[..S],
      &Block::<HeapElement<V>> { pos: new_pos, key: oram_key, value: heap_value },
    );

    for _ in 0..2 {
      let pos_to_evict = self.rng.random_range(0..self.data.max_n as PositionType);
      self.evict(pos_to_evict);
      self.update_min(pos_to_evict);
    }

    (new_pos, oram_key)
  }

  /// Deletes an element from the heap given it's timestamp and path.
  /// # Behavior
  /// * If the element is not in the heap, nothing happens.
  pub fn delete(&mut self, pos: PositionType, timestamp: K) {
    self.data.read_path_and_get_nodes(pos);
    remove_element(&mut self.data.stash, timestamp);
    self.data.evict_once_fast(pos);
    self.data.write_back_path(pos);
    self.update_min(pos);

    let pos_to_evict = self.rng.random_range(0..self.data.max_n as PositionType);
    self.evict(pos_to_evict);
    self.update_min(pos_to_evict);
  }

  /// Find and delete the minimum element from the heap.
  pub fn extract_min(&mut self) {
    let to_delete = self.find_min();
    self.delete(to_delete.pos, to_delete.key);
  }
}

#[cfg(test)]
mod tests {
  use std::{cmp::Reverse, collections::BinaryHeap};

  use super::*;

  #[test]
  fn test_insert_and_find_min() {
    let mut heap = Heap::new(4);

    heap.insert(10, 100);
    heap.insert(5, 50);
    heap.insert(20, 200);

    let min_element = heap.find_min();

    assert_eq!(min_element.value.key, 5);
    assert_eq!(min_element.value.value, 50);
  }

  #[test]
  fn test_insert_and_extract_min() {
    let mut heap = Heap::new(4);

    heap.insert(30, 300);
    heap.insert(10, 100);
    heap.insert(20, 200);

    let min_element = heap.find_min();
    assert_eq!(min_element.value.key, 10);
    assert_eq!(min_element.value.value, 100);

    heap.extract_min();

    let new_min_element = heap.find_min();
    assert_eq!(new_min_element.value.key, 20);
    assert_eq!(new_min_element.value.value, 200);
  }

  #[test]
  fn test_delete() {
    let mut heap = Heap::new(4);

    let _location = heap.insert(15, 150);

    let min_element = heap.find_min();

    heap.delete(min_element.pos, min_element.key);

    let min_element_after_delete = heap.find_min();
    assert!(min_element_after_delete.is_empty())
  }

  #[test]
  fn test_multiple_inserts_and_extracts() {
    let mut heap = Heap::new(8);

    for i in (1..=8).rev() {
      heap.insert(i, (i * 10) as u64);
    }

    let mut last_key = 0;
    for _i in 0..8 {
      let min_element = heap.find_min();
      assert!(!min_element.is_empty());
      assert!(min_element.value.key >= last_key);
      last_key = min_element.value.key;
      heap.extract_min();
    }
  }

  #[test]
  fn test_stress_with_many_operations() {
    let mut heap = Heap::new(32); // Larger heap for stress test
    let mut reference_heap = BinaryHeap::new();
    let operations = 100;

    // Track inserted items for potential deletion
    let mut inserted = Vec::new();

    for _ in 0..operations {
      let op = rand::rng().random_range(0..2);
      //heap.print_for_debug();
      match op {
        0 => {
          // Insert
          let key = rand::rng().random_range(0..1000);
          let value = key as u64 * 10;
          let _inserted_location = heap.insert(key, value);
          let min_element = heap.find_min();
          inserted.push(min_element);
          reference_heap.push(Reverse((key, value)));
        }
        1 => {
          // Extract min
          if !reference_heap.is_empty() {
            let min_element = heap.find_min();
            heap.extract_min();

            if let Some(Reverse((reference_min_key, reference_min_val))) = reference_heap.pop() {
              assert_eq!(
                min_element.value.key, reference_min_key,
                "Extract min returned incorrect key"
              );
              assert_eq!(
                min_element.value.value, reference_min_val,
                "Extract min returned incorrect value"
              );
            }
          }
        }
        _ => unreachable!(),
      }

      // Verify minimum is consistent
      if !inserted.is_empty() {
        let min_element = heap.find_min();
        if let Some(Reverse((reference_min_key, _))) = reference_heap.peek() {
          assert_eq!(
            min_element.value.key, *reference_min_key,
            "Heap minimum doesn't match reference after operation"
          );
        }
      }
    }
  }
}
