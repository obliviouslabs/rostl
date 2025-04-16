use std::mem::ManuallyDrop;
use std::{cmp, default};

use bytemuck::{Pod, Zeroable};
use rand::{rngs::ThreadRng, Rng};
use rods_oram::{
  circuit_oram::{remove_element, write_block_to_empty_slot, Block, CircuitORAM, S, Z},
  heap_tree::HeapTree,
  linear_oram::{oblivious_read_index, oblivious_write_index},
  prelude::{PositionType, K},
  recursive_oram::RecursivePositionMap,
};
use rods_primitives::{cmov_body, cxchg_body, impl_cmov_for_generic_pod};
use rods_primitives::{
  indexable::Length,
  traits::{Cmov, _Cmovbase},
};

// --- HeapV definition ---
#[derive(Clone, Copy, Debug, Zeroable)]
#[repr(C)]
pub struct HeapV<V>
where
  V: Cmov + Pod,
{
  pub key: K,
  pub value: V,
}
unsafe impl<V: Cmov + Pod> Pod for HeapV<V> {}
impl_cmov_for_generic_pod!(HeapV<V>; where V: Cmov + Pod);
impl<V: Cmov + Pod> Default for HeapV<V> {
  fn default() -> Self {
    HeapV { key: K::MAX, value: V::zeroed() }
  }
}

// --- Heap struct ---
#[derive(Debug)]
pub struct Heap<V>
where
  V: Cmov + Pod,
{
  pub data: CircuitORAM<HeapV<V>>,
  pub metadata: HeapTree<Block<HeapV<V>>>,
  pub rng: ThreadRng,
}

// --- Implement Length ---
impl<V> Length for Heap<V>
where
  V: Cmov + Pod,
{
  #[inline(always)]
  fn len(&self) -> usize {
    1usize << (self.data.h - 1)
  }
}

// --- Main Impl ---
impl<V> Heap<V>
where
  V: Cmov + Pod + Default + std::fmt::Debug,
{
  pub fn new(n: usize) -> Self {
    let data = CircuitORAM::new(n); // Initialize `data` first
    let default_heap_v = HeapV::<V> { key: K::MAX, value: V::default() };
    let default_value = Block::<HeapV<V>> { pos: 0, key: K::MAX, value: default_heap_v };
    let metadata = HeapTree::new_with(data.h, default_value); // Use `data.h` after `data` is initialized
    Self { data, metadata, rng: rand::rng() }
  }

  pub fn find_min(&mut self) -> (PositionType, K, K, V) {
    let min_node = self.metadata.get_node_by_index(0);
    let mut ret_pos = min_node.pos;
    let mut ret_oram_key = min_node.key;
    let mut ret_k = min_node.value.key;
    let mut ret_v = min_node.value.value;
    // println!("pos: {}, oram_key: {}, k: {}, v: {:?}", ret_pos, ret_oram_key, ret_k, ret_v);
    for elem in &self.data.stash[0..S] {
      let elem_key = if elem.is_empty() { K::MAX } else { elem.value.key };
      if elem_key < ret_k {
        ret_k = elem_key;
        ret_v = elem.value.value;
        ret_pos = elem.pos;
        ret_oram_key = elem.key;
      }
    }
    (ret_pos, ret_oram_key, ret_k, ret_v)
  }

  fn evict(&mut self, pos: PositionType) {
    self.data.read_path_and_get_nodes(pos);
    self.data.evict_once_fast(pos);
    self.data.write_back_path(pos);
  }

  fn print_for_debug(&self) {
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

  fn update_min(&mut self, pos: PositionType) {
    let data = &self.data;
    let mut h_index = self.metadata.height - 1;
    let metadata = &mut self.metadata;

    for elem in data.stash[S..(S + self.data.h * Z)].chunks(2).rev() {
      let elem0_key = if elem[0].is_empty() { K::MAX } else { elem[0].value.key };
      let elem1_key = if elem[1].is_empty() { K::MAX } else { elem[1].value.key };
      let mut new_metadata_node = elem[0];
      new_metadata_node.value.key = elem0_key;
      if elem0_key > elem1_key {
        new_metadata_node = elem[1];
      }

      if h_index == metadata.height - 1 {
        *metadata.get_path_at_depth_mut(h_index, pos) = new_metadata_node;
        h_index = h_index.saturating_sub(1);
        continue;
      }
      let one_child = metadata.get_path_at_depth(h_index + 1, pos);
      let the_other_child = metadata.get_the_other_child(h_index, pos);
      if one_child.value.key < new_metadata_node.value.key {
        new_metadata_node = *one_child;
      }
      if the_other_child.value.key < new_metadata_node.value.key {
        new_metadata_node = *the_other_child;
      }

      *metadata.get_path_at_depth_mut(h_index, pos) = new_metadata_node;
      h_index = h_index.saturating_sub(1);
    }
  }

  pub fn insert(&mut self, key: K, value: V) -> PositionType {
    let new_pos = self.rng.random_range(0..self.len() as PositionType);
    let oram_key: K = self.rng.random_range(0..usize::MAX);
    let heap_value = HeapV::<V> { key, value };
    write_block_to_empty_slot(
      &mut self.data.stash[..S],
      &Block::<HeapV<V>> { pos: new_pos, key: oram_key, value: heap_value },
    );
    //self.print_for_debug();
    for _ in 0..2 {
      let pos_to_evict = self.rng.random_range(0..self.len() as PositionType);
      //let pos_to_evict = 1;
      self.evict(pos_to_evict);
      //self.print_for_debug();
      self.update_min(pos_to_evict);
      //self.print_for_debug();
    }

    new_pos
  }

  pub fn delete(&mut self, pos: PositionType, oram_key: K) {
    self.data.read_path_and_get_nodes(pos);
    remove_element(&mut self.data.stash, oram_key);
    self.data.evict_once_fast(pos);
    self.data.write_back_path(pos);
    self.update_min(pos);

    let pos_to_evict = self.rng.random_range(0..self.len() as PositionType);
    self.evict(pos_to_evict);
    self.update_min(pos_to_evict);
  }

  pub fn extract_min(&mut self) {
    let (pos, oram_k, k, _) = self.find_min();
    self.delete(pos, oram_k);
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  // Heap<usize, u64> where K = usize and V = u64
  fn create_test_heap() -> Heap<u64> {
    Heap::new(4) // small test size
  }

  #[test]
  fn test_insert_and_find_min() {
    let mut heap = create_test_heap();

    heap.insert(10, 100);
    heap.insert(5, 50);
    heap.insert(20, 200);

    let (_pos, _, min_key, min_val) = heap.find_min();
    assert_eq!(min_key, 5);
    assert_eq!(min_val, 50);
  }

  #[test]
  fn test_insert_and_extract_min() {
    let mut heap = create_test_heap();

    heap.insert(30, 300);
    heap.insert(10, 100);
    heap.insert(20, 200);

    let (_pos, _, min_key, min_val) = heap.find_min();
    assert_eq!(min_key, 10);
    assert_eq!(min_val, 100);

    heap.extract_min();

    let (_, _, new_min_key, new_min_val) = heap.find_min();
    assert_eq!(new_min_key, 20);
    assert_eq!(new_min_val, 200);
  }

  #[test]
  fn test_delete() {
    let mut heap = create_test_heap();

    let pos = heap.insert(15, 150);
    let (_pos, oram_key, key, val) = heap.find_min();

    heap.delete(pos, oram_key);

    let (_, _, min_key, min_val) = heap.find_min();
    assert!(min_key != 15 || min_val != 150);
  }

  #[test]
  fn test_multiple_inserts_and_extracts() {
    let mut heap = create_test_heap();

    for i in (1..=5).rev() {
      heap.insert(i, (i * 10) as u64);
    }

    let mut last_val = 0;
    for _ in 0..5 {
      let (_, oram_key, key, val) = heap.find_min();
      assert!(val >= last_val);
      last_val = val;
      heap.extract_min();
    }
  }
}
