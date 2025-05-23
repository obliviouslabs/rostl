//! Implementation of [Circuit ORAM](https://eprint.iacr.org/2014/672.pdf)
//!
#![allow(clippy::needless_bitwise_bool)]

// UNDONE(git-8): This is needed to enforce the bitwise operations to not short circuit. Investigate if we should be using helper functions instead.
use bytemuck::{Pod, Zeroable};
use rods_primitives::{
  cmov_body, cxchg_body, impl_cmov_for_generic_pod,
  traits::{Cmov, _Cmovbase},
};

use crate::heap_tree::HeapTree;
use crate::prelude::{PositionType, DUMMY_POS, K};

/// Blocks per bucket
pub const Z: usize = 2;
/// Initial stash size
pub const S: usize = 20;
const EVICTIONS_PER_OP: usize = 2; // Evictions per operations

/// A block in the ORAM tree
/// # Invariants
/// If `pos == DUMMY_POS`, the block is empty, there are no guarantees about the key of value in that case.
/// If `pos != DUMMY_POS`, the block is full and the key and value are valid.
///
/// # Note
/// * It is wrong to assume anything about the block being empty or not based on the key, please use pos.
///
// #[repr(align(16))]
#[repr(C)]
#[derive(Clone, Copy, Zeroable)]
pub struct Block<V>
where
  V: Cmov + Pod,
{
  /// The position of the block
  pub pos: PositionType,
  /// The key of the block
  pub key: K,
  /// The data stored in the block
  pub value: V,
}
unsafe impl<V: Cmov + Pod> Pod for Block<V> {}

impl<T: Cmov + Pod> Default for Block<T> {
  fn default() -> Self {
    Self { pos: DUMMY_POS, key: 0, value: T::zeroed() }
  }
}

impl<T: Cmov + Pod + std::fmt::Debug> std::fmt::Debug for Block<T> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    if self.pos == DUMMY_POS {
      write!(f, ".")
    } else {
      write!(f, "Block {{ pos: {}, key: {}, value: {:?} }}", self.pos, self.key, self.value)
    }
  }
}

impl_cmov_for_generic_pod!(Block<V>;  where V: Cmov + Pod);

impl<V: Cmov + Pod> Block<V> {
  /// Checks if the block is empty or not
  pub const fn is_empty(&self) -> bool {
    self.pos == DUMMY_POS
  }
}

/// A bucket in the ORAM tree
#[repr(C)]
#[derive(Debug, Default, Clone, Copy, Zeroable)]
pub struct Bucket<V>([Block<V>; Z])
where
  V: Cmov + Pod;
unsafe impl<V: Cmov + Pod> Pod for Bucket<V> {}
impl_cmov_for_generic_pod!(Bucket<V>; where V: Cmov + Pod);

impl<V: Cmov + Pod> HeapTree<Bucket<V>> {
  /// Reads all the blocks in a path from the ORAM tree into an array in order:
  /// [Bucket0 (Root): [Block0, Block1], Bucket1 (Level1): [Block2, Block3], ...]
  #[inline]
  pub fn read_path(&mut self, path: PositionType, out: &mut [Block<V>]) {
    debug_assert!((path as usize) < (1 << self.height));
    debug_assert!(out.len() == self.height * Z);
    for i in 0..self.height {
      let index = self.get_index(i, path);
      let bucket = &self.tree[index];
      out[i * Z..(i + 1) * Z].copy_from_slice(&bucket.0);
    }
  }

  /// Writes a path to the ORAM tree, expects the input to be in the correct format, no checks are done:
  /// [Bucket0 (Root): [Block0, Block1], Bucket1 (Level1): [Block2, Block3], ...]
  #[inline]
  pub fn write_path(&mut self, path: PositionType, in_: &[Block<V>]) {
    debug_assert!((path as usize) < (1 << self.height));
    debug_assert!(in_.len() == self.height * Z);
    for i in 0..self.height {
      let index = self.get_index(i, path);
      let bucket = &mut self.tree[index];
      bucket.0.copy_from_slice(&in_[i * Z..(i + 1) * Z]);
    }
  }
}

/// The Circuit ORAM structure
///
/// # External Invariants
/// * Inv1 - There is an empty slot in the first S slots of the stash.
/// * use "Circuit ORAM invariants"
#[derive(Debug)]
pub struct CircuitORAM<V: Cmov + Pod> {
  /// Number of blocks, ilog2 of it is public via h
  pub max_n: usize,
  /// Height of the tree, public, single element tree has height 1, the depth of this element is 0
  pub h: usize,
  /// The stash: has size `S + h * Z`. First S blocks are the actual stash the rest are the path used during operations.
  pub stash: Vec<Block<V>>,
  /// The tree
  pub tree: HeapTree<Bucket<V>>,
  /// Evict counter
  pub evict_counter: PositionType,
}

#[inline]
fn read_and_remove_element<V: Cmov + Pod>(arr: &mut [Block<V>], k: K, ret: &mut V) -> bool {
  let mut rv = false;

  for item in arr {
    let matched = (!item.is_empty()) & (item.key == k);
    debug_assert!((!matched) | (!rv));

    ret.cmov(&item.value, matched);
    item.pos.cmov(&DUMMY_POS, matched);
    rv.cmov(&true, matched);
  }

  rv
}

/// Removes an element identified by key k from an array.
/// If the key is not in the array nothing happens.
/// Expects the element to appear at most once in the array.
#[inline]
pub fn remove_element<V: Cmov + Pod>(arr: &mut [Block<V>], k: K) -> bool {
  let mut rv = false;

  for item in arr {
    let matched = (!item.is_empty()) & (item.key == k);
    debug_assert!((!matched) | (!rv));

    item.pos.cmov(&DUMMY_POS, matched);
    rv.cmov(&true, matched);
  }

  rv
}

/// Writes a block to an empty slot in an array.
/// If there are no empty slots, nothing happens and returns false.
#[inline]
pub fn write_block_to_empty_slot<V: Cmov + Pod>(arr: &mut [Block<V>], val: &Block<V>) -> bool {
  let mut rv = false;

  for item in arr {
    let matched = (item.is_empty()) & (!rv);
    debug_assert!((!matched) | (!rv));

    item.cmov(val, matched);
    rv.cmov(&true, matched);
  }

  rv
}

/// Reverses the bits of a given number up to a specified number of bits.
///
/// # Arguments
///
/// * `num` - The number whose bits are to be reversed.
/// * `bits` - The number of bits to consider for the reversal.
///
/// # Returns
///
/// The number with its bits reversed.
#[inline]
pub fn reverse_bits(n: usize, bits: usize) -> usize {
  let mut result = 0;
  let mut value = n;

  for _ in 0..bits {
    result = (result << 1) | (value & 1);
    value >>= 1;
  }

  result
}

#[inline]
const fn common_suffix_length(a: PositionType, b: PositionType) -> u32 {
  let w = a ^ b;
  w.trailing_zeros()
}

impl<V: Cmov + Pod + Default + Clone + std::fmt::Debug> CircuitORAM<V> {
  /// Creates a new empty `CircuitORAM` instance with the given maximum number of blocks.
  ///
  /// # Arguments
  /// * `max_n` - The maximum number of blocks in the ORAM.
  ///
  /// # Returns
  /// A new instance of `CircuitORAM`.
  ///
  /// # Preconditions
  /// * `0 < max_n < (2**33)`
  pub fn new(max_n: usize) -> Self {
    // Eviction last level doesn't work for n=1 (due to last loop of evict function not checking if levels=1).
    debug_assert!(max_n > 1);
    debug_assert!(max_n <= u32::MAX as usize);

    let h = {
      let h0 = (max_n).ilog2() as usize;
      if (1 << h0) < max_n {
        h0 + 2
      } else {
        h0 + 1
      }
    };
    let tree = HeapTree::new(h);
    let stash = vec![Block::<V>::default(); S + h * Z];

    let max_n = 2usize.pow((h - 1) as u32);
    Self { max_n, h, stash, tree, evict_counter: 0 }
  }

  /// Creates a new `CircuitORAM` instance with the given maximum number of blocks, keys, values, and positions.
  ///
  /// # Arguments
  /// * `max_n` - The maximum number of blocks in the ORAM.
  /// * `keys` - A vector of keys for the blocks.
  /// * `values` - A vector of values for the blocks.
  /// * `positions` - A vector of positions for the blocks.
  ///
  /// # Returns
  /// A new instance of `CircuitORAM`.
  // UNDONE(git-9): Fast external-memory initialization
  pub fn new_with_positions_and_values(
    max_n: usize,
    keys: &[K],
    values: &[V],
    positions: &[PositionType],
  ) -> Self {
    let mut oram = Self::new(max_n);
    debug_assert!(keys.len() == values.len());
    debug_assert!(keys.len() == positions.len());
    debug_assert!(keys.len() <= max_n);

    for (i, ((key, value), pos)) in keys.iter().zip(values.iter()).zip(positions.iter()).enumerate()
    {
      oram.write_or_insert(i as PositionType, *pos, *key, *value);
    }
    oram
  }

  /// Reads a path to the end of the stash
  pub fn read_path_and_get_nodes(&mut self, pos: PositionType) {
    debug_assert!((pos as usize) < self.max_n);
    self.tree.read_path(pos, &mut self.stash[S..S + self.h * Z]);
  }

  /// Writes back the path at the end of the stash
  pub fn write_back_path(&mut self, pos: PositionType) {
    debug_assert!((pos as usize) < self.max_n);
    self.tree.write_path(pos, &self.stash[S..S + self.h * Z]);
  }

  /// Alg. 4 - EvictOnceFast(path) in `CircuitORAM` paper
  pub fn evict_once_fast(&mut self, pos: PositionType) {
    // UNDONE(git-10): Investigate using u8 and/or bitwise operations here instead of u32/bool cmov's
    // UNDONE(git-11): This only supports n<=32. Is it enough?
    //
    let mut deepest: [i32; 64] = [-1; 64];
    let mut deepest_idx: [i32; 64] = [0; 64];
    let mut target: [i32; 64] = [-1; 64];
    let mut has_empty: [bool; 64] = [false; 64];

    let mut src = -1;
    let mut dst: i32 = -1;

    // 1) First pass: (Alg 2 - PrepareDeepest in `CircuitORAM` paper).
    // dst is the same as goal in the paper
    // First level (including the stash):
    //
    for idx in 0..S + Z {
      let deepest_level = common_suffix_length(self.stash[idx].pos, pos) as i32;
      let deeper_flag = (!self.stash[idx].is_empty()) & (deepest_level > dst);
      dst.cmov(&deepest_level, deeper_flag);
      deepest_idx[0].cmov(&(idx as i32), deeper_flag);
    }
    src.cmov(&0, dst != -1);

    let mut idx = S + Z;
    // Remaining levels:
    //
    for i in 1..self.h {
      deepest[i].cmov(&src, dst >= i as i32);
      let mut bucket_deepest_level: i32 = -1;
      for _ in 0..Z {
        let deepest_level = common_suffix_length(self.stash[idx].pos, pos) as i32;
        let is_empty = self.stash[idx].is_empty();
        has_empty[i].cmov(&true, is_empty);

        let deeper_flag = (!is_empty) & (deepest_level > bucket_deepest_level);
        bucket_deepest_level.cmov(&deepest_level, deeper_flag);
        deepest_idx[i].cmov(&(idx as i32), deeper_flag);

        idx += 1;
      }

      let deepper_flag = bucket_deepest_level > dst;
      src.cmov(&(i as i32), deepper_flag);
      dst.cmov(&bucket_deepest_level, deepper_flag);
    }

    // 2) Second pass: (Alg 3 - PrepareTarget in CircuitORAM paper).
    //
    src = -1;
    dst = -1;
    for i in (1..self.h).rev() {
      let is_src = (i as i32) == src;
      target[i].cmov(&dst, is_src);
      src.cmov(&-1, is_src);
      dst.cmov(&-1, is_src);
      let change_flag = (((dst == -1) & has_empty[i]) | (target[i] != -1)) & (deepest[i] != -1);
      src.cmov(&deepest[i], change_flag);
      dst.cmov(&(i as i32), change_flag);
    }
    target[0].cmov(&dst, src == 0);

    // 3) Third pass: Actually move the data (end of Alg 4 - EvictOnceFast in CircuitORAM paper).
    //
    // First level (including the stash)
    let mut hold = Block::<V>::default();
    for idx in 0..S + Z {
      let is_deepest = deepest_idx[0] == idx as i32;
      let read_and_remove_flag = is_deepest & (target[0] != -1);
      hold.cmov(&self.stash[idx], read_and_remove_flag);
      self.stash[idx].pos.cmov(&DUMMY_POS, read_and_remove_flag);
    }
    dst = target[0];

    // Remaining levels except the last
    let mut idx = S + Z;
    for i in 1..(self.h - 1) {
      let has_target_flag = target[i] != -1;
      let place_dummy_flag = (i as i32 == dst) & (!has_target_flag);
      for _ in 0..Z {
        // case 0: level i is neither a dest and not a src
        //         hasTargetFlag = false, placeDummyFlag = false
        //         nothing will change
        // case 1: level i is a dest but not a src
        //         hasTargetFlag = false, placeDummyFlag = true
        //         hold will be swapped with each dummy slot
        //         after the first swap, hold will become dummy, and the
        //         subsequent swaps have no effect.
        // case 2: level i is a src but not a dest
        //         hasTargetFlag = true, placeDummyFlag = false
        //         hold must be dummy originally (eviction cannot carry two
        //         blocks). hold will be swapped with the slot that evicts to
        //         deepest.
        // case 3: level i is both a src and a dest
        //         hasTargetFlag = true, placeDummyFlag = false
        //         hold will be swapped with the slot that evicts to deepest,
        //         which fulfills both src and dest requirements.
        let is_deepest = deepest_idx[i] == idx as i32;
        let read_and_remove_flag = is_deepest & has_target_flag;
        let write_flag = (self.stash[idx].is_empty()) & place_dummy_flag;
        let swap_flag = read_and_remove_flag | write_flag;
        hold.cxchg(&mut self.stash[idx], swap_flag);
        idx += 1;
      }

      dst.cmov(&target[i], has_target_flag | place_dummy_flag);
    }

    // last level (this should not be called if h=1, but we just assert h>1)
    let place_dummy_flag = ((self.h - 1) as i32) == dst;
    let mut written = false;
    for _ in 0..Z {
      let write_flag = (self.stash[idx].is_empty()) & place_dummy_flag & (!written);
      written |= write_flag;
      self.stash[idx].cmov(&hold, write_flag);
      idx += 1;
    }
  }

  // Reads a path, performs evictions and writes back the path
  fn perform_eviction(&mut self, pos: PositionType) {
    debug_assert!((pos as usize) < self.max_n);
    self.read_path_and_get_nodes(pos);
    self.evict_once_fast(pos);
    self.write_back_path(pos);
  }

  /// (Alg. 6 - Evict Deterministic in `CircuitORAM` paper).
  /// # Postcondition
  /// * restores Inv1.
  fn perform_deterministic_evictions(&mut self) {
    // Empirically we found out this strategy works if reading and fetching a path is cheap
    for _ in 0..EVICTIONS_PER_OP {
      // let evict_pos = reverse_bits(self.evict_counter, self.h - 1);
      let evict_pos = self.evict_counter;
      self.perform_eviction(evict_pos);
      self.evict_counter = (self.evict_counter + 1) % (self.max_n as PositionType);
    }
    // UNDONE(git-12): Otherwise, if fetching a path is expensive, we should increase the stash size and do two evictions on the same path. (so read and write are only called once)

    // debug_assert that the stash has at least one empty slot:
    let mut ok = false;
    for elem in &self.stash[..S] {
      ok.cmov(&true, elem.is_empty());
    }
    debug_assert!(ok);
    // UNDONE(git-13): have a failure recovery path if it doesn't.
  }

  /// Reads a value from the ORAM.
  ///
  /// # Arguments
  /// * `pos` - The current position of the block.
  /// * `new_pos` - The new position of the block, should be uniformly random on the size of the ORAM.
  /// * `key` - The key of the block.
  /// * `ret` - The value to be read.
  ///
  /// # Returns
  /// * `true` if the element was found, `false` otherwise.
  /// # Behavior
  /// * If the element is not found, `ret` is not modified.
  pub fn read(&mut self, pos: PositionType, new_pos: PositionType, key: K, ret: &mut V) -> bool {
    debug_assert!((pos as usize) < self.max_n);
    debug_assert!((new_pos as usize) < self.max_n || new_pos == DUMMY_POS);

    self.read_path_and_get_nodes(pos);

    let found = read_and_remove_element(&mut self.stash, key, ret);
    let mut to_write = Block { pos: new_pos, key, value: *ret };
    to_write.pos.cmov(&DUMMY_POS, !found);
    write_block_to_empty_slot(&mut self.stash[..S], &to_write); // Succeeds due to Inv1.

    self.evict_once_fast(pos);
    self.write_back_path(pos);
    self.perform_deterministic_evictions();

    found
  }

  /// Writes a value to the ORAM.
  ///
  /// # Arguments
  /// * `pos` - The current position of the block.
  /// * `new_pos` - The new position of the block, should be uniformly random on the size of the ORAM.
  /// * `key` - The key of the block.
  /// * `val` - The value to be written.
  ///
  /// # Returns
  /// * `true` if the element was found and updated, `false` otherwise.
  /// # Behavior
  /// * If the element is not found, no modifications are made to the logical ORAM state.
  /// * If the element is found, it is updated with the new value.
  pub fn write(&mut self, pos: PositionType, new_pos: PositionType, key: K, val: V) -> bool {
    debug_assert!((pos as usize) < self.max_n);
    // I am not handling DUMMY_POS here, because it seems to never be the case where this would be needed.
    debug_assert!((new_pos as usize) < self.max_n);

    self.read_path_and_get_nodes(pos);

    let found = remove_element(&mut self.stash, key);

    let mut target_pos = DUMMY_POS;
    target_pos.cmov(&new_pos, found);

    write_block_to_empty_slot(
      &mut self.stash[..S],
      &Block::<V> { pos: target_pos, key, value: val },
    ); // Succeeds due to Inv1.

    self.evict_once_fast(pos);
    self.write_back_path(pos);
    self.perform_deterministic_evictions();

    found
  }

  /// Writes a value to the ORAM or inserts it if not found.
  ///
  /// # Arguments
  /// * `pos` - The current position of the block.
  /// * `new_pos` - The new position of the block, should be uniformly random on the size of the ORAM.
  /// * `key` - The key of the block.
  /// * `val` - The value to be written.
  ///
  /// # Returns
  /// * `true` if the element was found and updated, `false` if it was inserted.
  /// # Behavior
  /// * If the element is not found, it is inserted with the new value.
  /// * If the element is found, it is updated with the new value.
  pub fn write_or_insert(
    &mut self,
    pos: PositionType,
    new_pos: PositionType,
    key: K,
    val: V,
  ) -> bool {
    debug_assert!((pos as usize) < self.max_n);
    debug_assert!((new_pos as usize) < self.max_n || new_pos == DUMMY_POS);

    self.read_path_and_get_nodes(pos);
    // println!("{:?}", self.stash);

    let found = remove_element(&mut self.stash, key);
    // println!("{:?}", found);

    write_block_to_empty_slot(&mut self.stash[..S], &Block::<V> { pos: new_pos, key, value: val }); // Succeeds due to Inv1.
                                                                                                    // println!("{:?}", self.stash);

    self.evict_once_fast(pos);
    self.write_back_path(pos);
    self.perform_deterministic_evictions();

    found
  }

  /// Updates a value in the ORAM using a provided update function.
  /// If the element is not in the ORAM, it is inserted with the result of calling the update function on the default value.
  /// # Arguments
  /// * `pos` - The current position of the block.
  /// * `new_pos` - The new position of the block, should be uniformly random on the size of the ORAM.
  /// * `key` - The key of the block.
  /// * `update_func` - The function to update the value.
  ///
  /// # Returns
  /// * A tuple containing a boolean indicating if the element was found and the result of the update function.
  pub fn update<T, F>(
    &mut self,
    pos: PositionType,
    new_pos: PositionType,
    key: K,
    update_func: F,
  ) -> (bool, T)
  where
    F: FnOnce(&mut V) -> T,
  {
    debug_assert!((pos as usize) < self.max_n);
    debug_assert!((new_pos as usize) < self.max_n);

    self.read_path_and_get_nodes(pos);

    let mut val = V::default();
    let found = read_and_remove_element(&mut self.stash, key, &mut val);
    let rv = update_func(&mut val);

    write_block_to_empty_slot(&mut self.stash[..S], &Block::<V> { pos: new_pos, key, value: val }); // Succeeds due to Inv1.

    self.evict_once_fast(pos);
    self.write_back_path(pos);
    self.perform_deterministic_evictions();

    (found, rv)
  }

  #[cfg(test)]
  pub(crate) fn print_for_debug(&self) {
    println!("self.h: {}", self.h);
    println!("Stash: {:?}", self.stash);
    for i in 0..self.h {
      print!("Level {}: ", i);
      for j in 0..(1 << i) {
        let w_j = reverse_bits(j, i);
        print!(
          "{:?} ",
          self.tree.get_path_at_depth(
            i,
            reverse_bits(w_j * (1 << (self.h - 1 - i)), self.h - 1) as PositionType
          )
        );
      }
      println!();
    }
  }
}

#[cfg(test)]
mod tests {
  use std::vec;

  use super::*;
  use rand::{rng, Rng};

  fn assert_empty_stash(oram: &CircuitORAM<u64>) {
    for elem in &oram.stash[..S] {
      debug_assert!(elem.is_empty());
    }
  }

  #[test]
  fn test_print_for_debug() {
    let mut oram = CircuitORAM::<u64>::new(4);
    oram.perform_deterministic_evictions();
    assert_empty_stash(&oram);
    oram.print_for_debug();
    oram.write_or_insert(0, 0, 0, 0);
    oram.print_for_debug();
    oram.write_or_insert(0, 1, 1, 1);
    oram.print_for_debug();
    oram.write_or_insert(0, 2, 2, 2);
    oram.print_for_debug();
    oram.write_or_insert(0, 3, 3, 3);
    oram.print_for_debug();
    oram.perform_deterministic_evictions();
    oram.print_for_debug();
    oram.write_or_insert(0, 0, 4, 0);
    oram.print_for_debug();
    oram.write_or_insert(0, 1, 5, 1);
    oram.print_for_debug();
    oram.write_or_insert(0, 2, 6, 2);
    oram.print_for_debug();
    oram.write_or_insert(0, 3, 7, 3);
    oram.print_for_debug();
    oram.perform_deterministic_evictions();
    oram.print_for_debug();
    oram.write_or_insert(0, 0, 10, 0);
    oram.print_for_debug();
    oram.write_or_insert(0, 1, 11, 1);
    oram.print_for_debug();
    oram.write_or_insert(0, 2, 12, 2);
    oram.print_for_debug();
    oram.write_or_insert(0, 3, 13, 3);
    oram.print_for_debug();
    oram.perform_deterministic_evictions();
    // Currently none of these blocks will be evicted, as the first level is not seen as sepparate from the stash itself.
    oram.print_for_debug();
    oram.write_or_insert(0, 0, 20, 0);
    oram.print_for_debug();
    oram.write_or_insert(0, 1, 21, 1);
    oram.print_for_debug();
    oram.write_or_insert(0, 2, 22, 2);
    oram.print_for_debug();
    oram.write_or_insert(0, 3, 23, 3);
    oram.print_for_debug();
    oram.perform_deterministic_evictions();
    oram.perform_deterministic_evictions();
    oram.perform_deterministic_evictions();
    oram.perform_deterministic_evictions();
    oram.perform_deterministic_evictions();
    oram.print_for_debug();
  }

  #[test]
  fn test_circuitoram_simple() {
    let mut oram = CircuitORAM::<u64>::new(16);
    oram.perform_deterministic_evictions();
    assert_empty_stash(&oram);

    oram.write_or_insert(0, 0, 1, 1);
    assert_empty_stash(&oram);

    let mut v = 0;
    let found = oram.read(0, 0, 1, &mut v);
    assert!(found);
    assert_eq!(v, 1);
    assert_empty_stash(&oram);
    oram.print_for_debug();

    oram.write_or_insert(0, 0, 2, 2);
    assert_empty_stash(&oram);
    let found = oram.read(0, 0, 2, &mut v);
    assert!(found);
    assert_eq!(v, 2);
    assert_empty_stash(&oram);
    oram.print_for_debug();

    let found = oram.read(0, 0, 3, &mut v);
    assert!(!found);
    assert_empty_stash(&oram);
    oram.print_for_debug();

    oram.write_or_insert(0, 0, 1, 3);
    let found = oram.read(0, 0, 1, &mut v);
    assert!(found);
    assert_eq!(v, 3);
  }

  #[test]
  fn test_circuitoram_simple_2() {
    const TOTAL_KEYS: usize = 8;
    let mut oram = CircuitORAM::<u64>::new(TOTAL_KEYS);
    let mut val = 0;
    let found = oram.write_or_insert(0, 4, 0, 123);
    oram.print_for_debug();
    assert!(!found);
    let found = oram.read(4, 7, 0, &mut val);
    oram.print_for_debug();
    assert!(found);
    assert_eq!(val, 123);
  }

  fn test_circuitoram_repetitive_generic<const TOTAL_KEYS: PositionType>() {
    let mut oram = CircuitORAM::<u64>::new(TOTAL_KEYS as usize);
    let mut pmap = vec![0; TOTAL_KEYS as usize];
    let mut vals = vec![0; TOTAL_KEYS as usize];
    let mut used = vec![false; TOTAL_KEYS as usize];
    let mut rng = rng();

    for _ in 0..2_000 {
      let new_pos = rng.random_range(0..TOTAL_KEYS);
      let key = 0;
      rng.random_range(0..TOTAL_KEYS);
      let val = rng.random::<u64>();
      let op = rng.random_range(0..3);
      // println!("op: {}, key: {}, val: {}, new_pos: {}", op, key, val, new_pos);
      // oram.print_for_debug();
      if op == 0 {
        let mut v = 0;
        let found = oram.read(pmap[key], new_pos, key as K, &mut v);
        assert_eq!(found, used[key]);
        if used[key] {
          assert_eq!(v, vals[key]);
        }
      } else if op == 1 {
        let found = oram.write(pmap[key], new_pos, key as K, val);
        assert_eq!(found, used[key]);
        vals[key] = val;
      } else if op == 2 {
        let found = oram.write_or_insert(pmap[key], new_pos, key as K, val);
        assert_eq!(found, used[key]);
        used[key] = true;
        vals[key] = val;
      } else if op == 3 {
        let found = oram.update(pmap[key], new_pos, key as K, |v| {
          *v = val;
          *v
        });
        assert_eq!(found.0, used[key]);
        if used[key] {
          assert_eq!(found.1, vals[key]);
        }
        used[key] = true;
        vals[key] = val;
      }

      pmap[key] = new_pos;
    }
  }

  //this test is failing now, but ORAM still works with the elements that should be moved to root left in stash
  //UNDONE(git-62): Fix this test
  // #[test]
  // fn test_eviction_once_fast() {
  //   let mut oram = CircuitORAM::<u32>::new(4);
  //   write_block_to_empty_slot(
  //     &mut oram.stash[..S],
  //     &Block::<u32> { pos:1, key: 100, value: 100 },
  //   );
  //   oram.evict_once_fast(0);
  //   assert!(oram.stash[0].is_empty());
  // }

  #[test]
  fn test_circuitoram_repetitive() {
    test_circuitoram_repetitive_generic::<8>();
    test_circuitoram_repetitive_generic::<16>();
    test_circuitoram_repetitive_generic::<1024>();
  }

  // UNDONE(git-24): Add a test to visualize circuit oram failure probability.
}
