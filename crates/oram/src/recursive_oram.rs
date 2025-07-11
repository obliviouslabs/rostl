//! Implements the recursive ORAM Technique
// UNDONE(git-17): Cite a paper or give a link in our docs explaining the recursive ORAM technique

use bytemuck::{Pod, Zeroable};
use core::mem::size_of;
use rand::rngs::ThreadRng;
use rand::{rng, Rng};
use rostl_primitives::traits::Cmov;
use rostl_primitives::utils::{max, min};
use rostl_primitives::{cmov_body, cxchg_body, impl_cmov_for_pod, traits::_Cmovbase};

use static_assertions::const_assert;

use crate::circuit_oram::CircuitORAM;
use crate::linear_oram::{oblivious_read_update_index, LinearORAM};
use crate::prelude::{PositionType, DUMMY_POS, K};

// UNDONE(git-25): Optimize these constants:
#[cfg(not(test))]
const LEVEL_0_BUCKETS: usize = 128;
#[cfg(not(test))]
const FAN_OUT: usize = max(2, 64 / size_of::<PositionType>());

// These are for making tests reasonably faster:
#[cfg(test)]
const LEVEL_0_BUCKETS: usize = 8;
#[cfg(test)]
const FAN_OUT: usize = max(2, 32 / size_of::<PositionType>());

const LINEAR_MAP_SIZE: usize = LEVEL_0_BUCKETS * FAN_OUT;
// const LINEAR_MAP_SIZE: usize = 4; // For debug
// const FAN_OUT: usize = 4; // For debug

const_assert!(LINEAR_MAP_SIZE.is_power_of_two());
const LEVEL0_BITS: usize = LINEAR_MAP_SIZE.ilog2() as usize;
const MASK0: usize = LINEAR_MAP_SIZE - 1;

const_assert!(FAN_OUT.is_power_of_two());
const LEVELN_BITS: usize = FAN_OUT.ilog2() as usize;
const MASKN: usize = FAN_OUT - 1;

#[repr(transparent)]
#[derive(Debug, Clone, Copy, Zeroable, Pod)]
struct InternalNode([PositionType; FAN_OUT]);
impl_cmov_for_pod!(InternalNode);

impl Default for InternalNode {
  fn default() -> Self {
    Self([DUMMY_POS; FAN_OUT])
  }
}

// UNDONE(git-18): Theoretically, the top position map levels could use less bits. Figure out if this would be efficient in practice.
/// An efficient position map for an ORAM where the key only has values from 0 to n-1
/// The position map is implemented as a linear ORAM for the first level
/// and a series of recursive ORAMs for the remaining levels
#[derive(Debug)]
pub struct RecursivePositionMap {
  /// The first level
  linear_oram: LinearORAM<InternalNode>,
  /// Remaining levels
  recursive_orams: Vec<CircuitORAM<InternalNode>>,
  /// The number of positions in the ORAM
  pub n: usize,
  /// The depth of the ORAM,
  /// i.e. the number of levels in the recursive ORAM
  h: usize, // public
  /// Thread local rng
  rng: ThreadRng,
}

impl RecursivePositionMap {
  /// Creates a new `RecursivePositionMap` with the given size `n`.
  // UNDONE(git-9): Fast external-memory initialization
  // UNDONE(git-19): Optimize this function
  pub fn new(n: usize) -> Self {
    debug_assert!(n > 0);
    let mut h: usize;
    let mut rng = rng();
    let l0_buckets = min(n.div_ceil(FAN_OUT), LEVEL_0_BUCKETS);

    let mut first_level: LinearORAM<InternalNode> = if n <= LINEAR_MAP_SIZE {
      h = 0;
      LinearORAM::new(l0_buckets)
    } else {
      h = (n / LINEAR_MAP_SIZE).ilog(FAN_OUT) as usize;
      LinearORAM::new(LEVEL_0_BUCKETS)
    };
    if LINEAR_MAP_SIZE * FAN_OUT.pow(h as u32) < n {
      h += 1;
    }
    debug_assert!(LINEAR_MAP_SIZE * FAN_OUT.pow(h as u32) >= n);

    let mut data_maps = Vec::with_capacity(h);
    let mut curr = min(LINEAR_MAP_SIZE, n);
    for _ in 0..h {
      data_maps.push(CircuitORAM::new(curr));
      curr *= FAN_OUT;
    }

    // UNDONE(git-19): Optimize this (make it cache efficient)
    let max_out_pos: PositionType = min(curr, n) as PositionType;
    let mut positions_maps_for_level: Vec<PositionType> =
      (0..curr).map(|_| rng.random_range(0..max_out_pos)).collect();
    for i in (0..h).rev() {
      curr /= FAN_OUT;
      let keys = (0..curr).map(|i| i as K).collect::<Vec<K>>();
      let mut values = vec![InternalNode::default(); curr];

      for j in 0..curr {
        for k in 0..FAN_OUT {
          values[j].0[k] = positions_maps_for_level[j * FAN_OUT + k];
        }
      }
      positions_maps_for_level =
        (0..curr as PositionType).map(|_| rng.random_range(0..curr as PositionType)).collect();
      data_maps[i] =
        CircuitORAM::new_with_positions_and_values(curr, &keys, &values, &positions_maps_for_level);
    }

    // Write the first level, bucket by bucket:
    //
    for i in 0..l0_buckets {
      let mut inner = InternalNode::default();
      for j in 0..FAN_OUT {
        let pos = i * FAN_OUT + j;
        if pos >= curr {
          break;
        }
        inner.0[j] = positions_maps_for_level[pos];
      }
      first_level.data[i] = inner;
    }

    Self { linear_oram: first_level, recursive_orams: data_maps, n, h, rng }
  }

  /// Accesses the position of a key `k` and updates it to `new_pos`.
  ///
  /// # Arguments
  ///
  /// * `k` - The key whose position is to be accessed.
  /// * `new_pos` - The new position to update the key to.
  ///
  /// # Returns
  ///
  /// The previous position of the key.
  pub fn access_position(&mut self, k: K, new_pos: PositionType) -> PositionType {
    debug_assert!(new_pos < self.n as PositionType);
    let mut ret: PositionType = PositionType::default();
    let mut k = k;
    let mut curr_max_pos = 1;
    let mask0 = k & MASK0;
    let mut curr_k = mask0;
    k >>= LEVEL0_BITS;
    curr_max_pos <<= LEVEL0_BITS;

    let mut new_curr_pos: PositionType =
      if self.h == 0 { new_pos } else { self.rng.random_range(0..curr_max_pos) };

    let level0_bucket_idx = curr_k >> LEVELN_BITS;
    let mut level0_bucket = InternalNode::default();
    self.linear_oram.read(level0_bucket_idx, &mut level0_bucket);
    oblivious_read_update_index(&mut level0_bucket.0, curr_k & MASKN, &mut ret, new_curr_pos);
    self.linear_oram.write(level0_bucket_idx, level0_bucket);

    // let mut pos = self.linear_oram.access_position(k, new_pos);
    for i in 0..self.h {
      let mask = k & MASKN;
      k >>= LEVELN_BITS;
      curr_max_pos <<= LEVELN_BITS;

      let pos = ret;
      let next_curr_pos =
        if self.h == i + 1 { new_pos } else { self.rng.random_range(0..curr_max_pos) };

      let (_found, nextpos) =
        self.recursive_orams[i].update(pos, new_curr_pos, curr_k, |node: &mut InternalNode| {
          let mut ret = DUMMY_POS;
          oblivious_read_update_index(&mut node.0, mask, &mut ret, next_curr_pos);
          ret
        });
      debug_assert!(_found);
      new_curr_pos = next_curr_pos;

      ret = nextpos;
      curr_k <<= LEVELN_BITS;
      curr_k |= mask;
    }

    ret
  }

  #[cfg(test)]
  pub(crate) fn print_for_debug(&self) {
    println!("Linear ORAM:");
    self.linear_oram.print_for_debug();
    for i in 0..self.h {
      println!("Level {} ORAM:", i);
      self.recursive_orams[i].print_for_debug();
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_recursive_position_map_small() {
    let n = LINEAR_MAP_SIZE / 2 + 1;
    let mut pos_map = RecursivePositionMap::new(n);
    assert_eq!(pos_map.h, 0);
    assert_eq!(pos_map.linear_oram.data.len(), n.div_ceil(FAN_OUT));
    for i in 0..n {
      pos_map.access_position(i, i as PositionType);
    }
    for i in 0..n {
      assert_eq!(pos_map.access_position(i, i as PositionType), i as PositionType);
    }
  }

  #[test]
  fn test_recursive_position_map_onelevel() {
    let n = LINEAR_MAP_SIZE * FAN_OUT;
    let mut pos_map = RecursivePositionMap::new(n);
    assert_eq!(pos_map.h, 1);
    assert_eq!(pos_map.linear_oram.data.len(), LEVEL_0_BUCKETS);
    pos_map.print_for_debug();
    for i in 0..n {
      pos_map.access_position(i, i as PositionType);
    }
    for i in 0..n {
      assert_eq!(pos_map.access_position(i, i as PositionType), i as PositionType);
    }
  }

  fn test_recursive_position_generic<const TOTAL_KEYS: usize>() {
    let mut pos_map = Box::new(RecursivePositionMap::new(TOTAL_KEYS));
    let mut rng = rng();
    let mut pmap = vec![0; TOTAL_KEYS];
    let mut used = vec![false; TOTAL_KEYS];

    for _i in 0..2000 {
      let k = rng.random_range(0..TOTAL_KEYS);
      let new_pos = rng.random_range(0..TOTAL_KEYS as PositionType);
      let old_pos = pos_map.access_position(k, new_pos);
      if used[k] {
        assert_eq!(pmap[k], old_pos);
      }
      pmap[k] = new_pos;
      used[k] = true;
    }
  }

  #[test]
  fn test_recursive_position_map_multiple() {
    const TOTAL_KEYS_0: usize = LINEAR_MAP_SIZE / 2 + 1;
    test_recursive_position_generic::<TOTAL_KEYS_0>();
    test_recursive_position_generic::<LINEAR_MAP_SIZE>();
    const TOTAL_KEYS_1: usize = LINEAR_MAP_SIZE * FAN_OUT;
    test_recursive_position_generic::<TOTAL_KEYS_1>();
    const TOTAL_KEYS_2: usize = LINEAR_MAP_SIZE * FAN_OUT * FAN_OUT;
    test_recursive_position_generic::<TOTAL_KEYS_2>();
  }
}
