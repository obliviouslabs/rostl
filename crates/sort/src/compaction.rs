//! Implements oblivious compaction algorithms.
//! 

use rods_primitives::traits::{Cmov, CswapIndex};
// use rods_primitives::indexable::{Indexable, Length};

/// Compacts an array of length n in place using nlogn oblivious compaction.
pub fn compact<T, F>(arr: &mut [T], is_dummy: F) -> usize
where 
  F: Fn(&T) -> bool,
  T: Cmov + Copy {
  let l2len = arr.len().next_power_of_two().trailing_zeros() as usize;
  let mut csum = vec![0; arr.len()+1];
  csum[0] = 0;
  let pred = is_dummy(&arr[0]);
  csum[0].cmov(&0, pred);
  for i in 1..arr.len() {
    csum[i] = csum[i-1];
    let pred = is_dummy(&arr[i]);
    let incred = csum[i] + 1;
    csum[i].cmov(&(incred), pred);
  }
  let ret = arr.len()-csum[arr.len()-1];

  for i in 0..l2len {
    for j in 0..(arr.len()-l2len) {
      let offset = 1 << i;
      let a = j;
      let b = j + offset;
      let pred = (csum[j] & offset) != 0;
      arr.cswap(a, b, pred);
      let newacsum = csum[b] - offset;
      csum[a].cmov(&newacsum, pred);
      csum[b].cmov(&0, pred);
    }
  }

  ret
}
