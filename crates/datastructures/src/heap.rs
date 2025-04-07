#![allow(missing_docs)]
#![allow(unused_imports)]
#![allow(deprecated)]
use std::cmp;
use std::mem::ManuallyDrop;

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
#[derive(Clone, Copy, Debug, Default, Zeroable)]
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

// --- Heap struct ---
#[derive(Debug)]
pub struct Heap<V>
where
  V: Cmov + Pod,
{
  // pub data: CircuitORAM<HeapV<V>>,
  // pub metadata: HeapTree<Block<HeapV<V>>>,
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
    let metadata = HeapTree::new(data.h); // Use `data.h` after `data` is initialized
    Self { data, metadata, rng: rand::thread_rng() }
  }

  pub fn find_min(&mut self) -> (PositionType, K, V) {
    let min_node = self.metadata.get_node_by_index(0);
    let ret_pos = min_node.pos;
    let ret_k = min_node.key;
    let ret_v = min_node.value.value;
    (ret_pos, ret_k, ret_v)
  }

  fn evict(&mut self, pos: PositionType) {
    println!("in the evict function 1");
    self.print_for_debug();
    self.data.read_path_and_get_nodes(pos);
    println!("in the evict function 2");
    self.print_for_debug();
    self.data.evict_once_fast(pos);
    println!("in the evict function 3");
    self.print_for_debug();
    self.data.write_back_path(pos);
    println!("in the evict function 4");
    self.print_for_debug();
  }

  fn print_for_debug(&self) {
    let data = &self.data;
    println!("Stash: {:?}", data.stash);
    for i in 0..data.h {
      print!("Level {}: ", i);
      for j in 0..(1 << i) {
        print!("{:?} ", data.tree.get_path_at_depth(i, j << (data.h - 1 - i)));
      }
      println!();
    }
  }

  fn update_min(&mut self, pos: PositionType) {
    let data = &self.data;
    let mut h_index = self.metadata.height - 1;
    let metadata = &mut self.metadata;

    for elem in data.stash[S..(S + self.data.h * Z)].chunks(2).rev() {
      let metadata_index = metadata.get_index(h_index, pos);
      let elem0_key = if elem[0].is_empty() { K::MAX } else { elem[0].value.key };
      let elem1_key = if elem[1].is_empty() { K::MAX } else { elem[1].value.key };

      println!("left key: {}, right key: {}", elem0_key, elem1_key);

      let (chosen, _) =
        if elem0_key < elem1_key { (&elem[0], &elem[1]) } else { (&elem[1], &elem[0]) };

      let mut new_metadata_node = *chosen;

      if h_index == metadata.height - 1 {
        *metadata.get_path_at_depth_mut(h_index, pos) = new_metadata_node;
        continue;
      }

      let left_child = metadata.get_left_child_index(metadata_index);
      let right_child = metadata.get_right_child_index(metadata_index);

      if left_child.key < new_metadata_node.key {
        new_metadata_node = *left_child;
      }
      if right_child.key < new_metadata_node.key {
        new_metadata_node = *right_child;
      }

      *metadata.get_path_at_depth_mut(h_index, pos) = new_metadata_node;
      h_index = h_index.saturating_sub(1);
    }
  }

  pub fn insert(&mut self, key: K, value: V) -> PositionType {
    let new_pos = self.rng.gen_range(0..self.len() as PositionType);
    let oram_key: K = self.rng.gen_range(0..usize::MAX);
    println!("lentgh: {}, pos: {}, oram_key: {}", self.len(), new_pos, oram_key);
    let heap_value = HeapV::<V> { key, value };
    write_block_to_empty_slot(
      &mut self.data.stash[..S],
      &Block::<HeapV<V>> { pos: new_pos, key: oram_key, value: heap_value },
    );
    println!("stash after write block to empty slot");
    self.print_for_debug();
    for _ in 0..2 {
      let pos_to_evict = self.rng.gen_range(0..self.len() as PositionType);
      println!("pos_to_evict: {}", pos_to_evict);
      self.evict(pos_to_evict);
      println!("stash after evict slot");
      self.print_for_debug();
      println!("updating min on the path: {}", pos_to_evict);
      self.update_min(pos_to_evict);
    }

    new_pos
  }

  pub fn delete(&mut self, pos: PositionType, oram_key: K) {
    self.data.read_path_and_get_nodes(pos);
    remove_element(&mut self.data.stash, oram_key);
    self.data.evict_once_fast(pos);
    self.data.write_back_path(pos);
    self.update_min(pos);

    let pos_to_evict = self.rng.gen_range(0..self.len() as PositionType);
    self.evict(pos_to_evict);
    self.update_min(pos_to_evict);
  }

  pub fn extract_min(&mut self) {
    let (pos, k, _) = self.find_min();
    self.delete(pos, k);
  }
}

// #[cfg(test)]
// mod tests {
//     use super::*;

//     // Heap<usize, u64> where K = usize and V = u64
//     fn create_test_heap() -> Heap<u64> {
//         Heap::new(4) // small test size
//     }

//     // #[test]
//     // fn test_insert_and_find_min() {
//     //     let mut heap = create_test_heap();

//     //     heap.insert(10, 100);
//     //     let (_pos, min_key, min_val) = heap.find_min();
//     //     println!("min_key: {}, min_val: {}", min_key, min_val);
//     //     heap.insert(5, 50);
//     //     let (_pos, min_key, min_val) = heap.find_min();
//     //     println!("min_key: {}, min_val: {}", min_key, min_val);
//     //     heap.insert(20, 200);
//     //     let (_pos, min_key, min_val) = heap.find_min();
//     //     println!("min_key: {}, min_val: {}", min_key, min_val);

//     //     let (_pos, min_key, min_val) = heap.find_min();
//     //     assert_eq!(min_key, 5);
//     //     assert_eq!(min_val, 50);
//     // }

//     // #[test]
//     // fn test_insert_and_extract_min() {
//     //     let mut heap = create_test_heap();

//     //     heap.insert(30, 300);
//     //     heap.insert(10, 100);
//     //     heap.insert(20, 200);

//     //     let (_pos, min_key, min_val) = heap.find_min();
//     //     assert_eq!(min_key, 10);
//     //     assert_eq!(min_val, 100);

//     //     heap.extract_min();

//     //     let (_, new_min_key, new_min_val) = heap.find_min();
//     //     assert_ne!(new_min_key, 10);
//     //     assert_ne!(new_min_val, 100);
//     // }

//     // #[test]
//     // fn test_delete() {
//     //     let mut heap = create_test_heap();

//     //     let pos = heap.insert(15, 150);
//     //     let (_pos, key, val) = heap.find_min();

//     //     heap.delete(pos, key);

//     //     let (_, min_key, min_val) = heap.find_min();
//     //     assert!(min_key != 15 || min_val != 150);
//     // }

//     // #[test]
//     // fn test_multiple_inserts_and_extracts() {
//     //     let mut heap = create_test_heap();

//     //     for i in (1..=5).rev() {
//     //         heap.insert(i, (i * 10) as u64);
//     //     }

//     //     let mut last_val = 0;
//     //     for _ in 0..5 {
//     //         let (_, key, val) = heap.find_min();
//     //         assert!(val >= last_val);
//     //         last_val = val;
//     //         heap.extract_min();
//     //     }
//     // }
// }
