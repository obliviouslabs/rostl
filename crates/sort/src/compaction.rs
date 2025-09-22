//! Implements oblivious compaction algorithms.
//!

use assume::assume;
use rostl_primitives::{
  traits::{Cmov, CswapIndex},
  utils::get_smaller_or_equal_power_of_two,
};
// use rostl_primitives::indexable::{Indexable, Length};

/// Computes the prefix sum of valid elements in arr.
/// # Behavior
/// * returns `ret` - the prefix sum array of length `arr.len() + 1`
/// # Oblivious
/// * Fully data-independent memory access pattern.
/// * Leaks: `arr.len()` - the full length of the original array
pub fn compute_prefix_sum<T, F>(arr: &[T], is_dummy: F) -> Vec<usize>
where
  F: Fn(&T) -> bool,
{
  let size = arr.len();
  let mut sarr = vec![0; size + 1];
  for i in 0..size {
    let mut adder = 1usize;
    adder.cmov(&0, is_dummy(&arr[i]));
    sarr[i + 1] = sarr[i] + adder;
  }
  sarr
}

/// Stably compacts an array `arr` of length n in place using nlogn oblivious compaction.
/// Uses `https://arxiv.org/pdf/1103.5102`
/// # Behavior
/// * returns `ret` - the number of non-dummy elements (new real length of the array).
/// * first `ret` elements of `arr` are the non-dummy elements in the same order as they were in the original array.
/// * the rest of the elements in `arr` are dummy elements.
/// # Oblivious
/// * Fully data-independent memory access pattern.
/// * Leaks: `arr.len()` - the full length of the original array
#[deprecated(note = "use compact instead, it's faster")]
pub fn compact_goodrich<T, F>(arr: &mut [T], is_dummy: F) -> usize
where
  F: Fn(&T) -> bool,
  T: Cmov + Copy,
{
  if arr.is_empty() {
    return 0;
  }
  let l2len = arr.len().next_power_of_two().trailing_zeros() as usize;
  let mut csum = vec![0; arr.len()];
  let mut dummy_count = 0;

  csum[0] = 0;
  let pred = is_dummy(&arr[0]);
  dummy_count.cmov(&1, pred);
  for i in 1..arr.len() {
    csum[i] = 0;
    let pred = is_dummy(&arr[i]);
    dummy_count.cmov(&(dummy_count + 1), pred);
    csum[i].cmov(&(dummy_count), !pred);
  }
  let ret = arr.len() - dummy_count;

  for i in 0..l2len {
    let offset = 1 << i;
    for j in 0..(arr.len() - offset) {
      let a = j;
      let b = j + offset;
      let pred = (csum[b] & offset) != 0;
      arr.cswap(a, b, pred);
      let newacsum = csum[b].wrapping_sub(offset);
      csum[a].cmov(&newacsum, pred);
      csum[b].cmov(&0, pred);
    }
  }

  ret
}

/// Compacts arr, as marked by the prefixsum payload, to an offset z.
/// # Requires
/// * `arr.len()` is a power of two.
/// * `payload.len() == arr.len() + 1`
/// * payload is a prefix sum of valid elements in arr.
/// * `0 <= z < arr.len()`
fn compact_payload_offset<T>(arr: &mut [T], payload: &[usize], z: usize)
where
  T: Cmov + Copy,
{
  assume!(unsafe: arr.len()+1 == payload.len());
  let n = arr.len();
  let half_n = n / 2;
  let m = payload[half_n] - payload[0];
  if n == 2 {
    let should_swap = ((!m) & (payload[2] - payload[1])) != z;
    arr.cswap(0, 1, should_swap);
    return;
  }
  let zleft = z % half_n;
  let zright = (z + m) % half_n;
  compact_payload_offset(&mut arr[..half_n], &payload[..half_n + 1], zleft);
  compact_payload_offset(&mut arr[half_n..], &payload[half_n..], zright);

  let s_a = zleft + m >= half_n;
  let s_b = z >= half_n;
  let s = s_a ^ s_b;

  for i in 0..half_n {
    let left = i;
    let right = i + half_n;
    let cond = s ^ (i >= zright);
    assume!(unsafe: left < arr.len());
    assume!(unsafe: right < arr.len());
    arr.cswap(left, right, cond);
  }
}

/// Stably compacts an array `arr` of length n using oblivious compaction.
/// The payload array `payload` is the prefix sum of valid elements.
/// Uses `https://eprint.iacr.org/2022/1333.pdf`
/// # Requires
/// * `payload.len() == arr.len() + 1`
/// * payload is a prefix sum of valid elements in arr.
/// * first elements of `arr` are the non-dummy elements in the same order as they were in the original array.
/// * the rest of the elements in `arr` are the dummy elements in no particular order.
/// # Oblivious
/// * Fully data-independent memory access pattern.
/// * Leaks: `arr.len()` - the full length of the original array
pub fn compact_payload<T>(arr: &mut [T], payload: &[usize])
where
  T: Cmov + Copy,
{
  assume!(unsafe: arr.len() + 1 == payload.len());
  let n = arr.len();
  if n <= 1 {
    return;
  }

  let n1 = get_smaller_or_equal_power_of_two(n);
  let n2 = n - n1;

  if n2 == 0 {
    compact_payload_offset(arr, payload, 0);
    return;
  }

  let m = payload[n2] - payload[0];
  compact_payload(arr[..n2].as_mut(), &payload[..n2 + 1]);
  compact_payload_offset(arr[n2..].as_mut(), &payload[n2..], (n1 - n2 + m) % n1);

  for i in 0..n2 {
    let left = i;
    let right = i + n1;
    assume!(unsafe: left < arr.len());
    assume!(unsafe: right < arr.len());
    arr.cswap(left, right, i >= m);
  }
}

/// Stably compacts an array `arr` of length n using oblivious compaction.
/// The payload array `payload` is the prefix sum of valid elements.
/// Uses `https://eprint.iacr.org/2022/1333.pdf`
/// # Requires
/// # Behavior
/// * returns `ret` - the number of non-dummy elements (new real length of the array).
/// * first `ret` elements of `arr` are the non-dummy elements in the same order as they were in the original array.
/// * the rest of the elements in `arr` are the dummy elements in no particular order.
/// # Oblivious
/// * Fully data-independent memory access pattern.
/// * Leaks: `arr.len()` - the full length of the original array
/// # Returns the number of non-dummy elements in the array after compaction.
pub fn compact<T, F>(arr: &mut [T], is_dummy: F) -> usize
where
  F: Fn(&T) -> bool,
  T: Cmov + Copy,
{
  let payload = compute_prefix_sum(arr, is_dummy);
  compact_payload(arr, &payload);
  payload[payload.len() - 1]
}

fn distribute_payload_offset<T>(arr: &mut [T], payload: &[usize], z: usize)
where
  T: Cmov + Copy,
{
  assume!(unsafe: arr.len()+1 == payload.len());
  let n = arr.len();
  let half_n = n / 2;
  let m = payload[half_n] - payload[0];
  if n == 2 {
    let should_swap = ((!m) & (payload[2] - payload[1])) != z;
    arr.cswap(0, 1, should_swap);
    return;
  }
  let zleft = z % half_n;
  let zright = (z + m) % half_n;
  let s_a = zleft + m >= half_n;
  let s_b = z >= half_n;
  let s = s_a ^ s_b;

  for i in 0..half_n {
    let left = i;
    let right = i + half_n;
    let cond = s ^ (i >= zright);
    assume!(unsafe: left < arr.len());
    assume!(unsafe: right < arr.len());
    arr.cswap(left, right, cond);
  }
  distribute_payload_offset(&mut arr[..half_n], &payload[..half_n + 1], zleft);
  distribute_payload_offset(&mut arr[half_n..], &payload[half_n..], zright);
}

/// Distributes the elements of arr according to the prefix sum payload (reverse of compaction for the same payload).
/// The payload array `payload` is the prefix sum of valid elements.
/// Uses `https://eprint.iacr.org/2022/1333.pdf`
pub fn distribute_payload<T>(arr: &mut [T], payload: &[usize])
where
  T: Cmov + Copy,
{
  assume!(unsafe: arr.len() + 1 == payload.len());
  let n = arr.len();
  if n <= 1 {
    return;
  }

  let n1 = get_smaller_or_equal_power_of_two(n);
  let n2 = n - n1;

  if n2 == 0 {
    distribute_payload_offset(arr, payload, 0);
    return;
  }

  let m = payload[n2] - payload[0];

  for i in 0..n2 {
    let left = i;
    let right = i + n1;
    assume!(unsafe: left < arr.len());
    assume!(unsafe: right < arr.len());
    arr.cswap(left, right, i >= m);
  }

  distribute_payload(arr[..n2].as_mut(), &payload[..n2 + 1]);
  distribute_payload_offset(arr[n2..].as_mut(), &payload[n2..], (n1 - n2 + m) % n1);
}

#[cfg(test)]
#[allow(deprecated)]
mod tests {
  use rand::Rng;

  use super::*;

  #[test]
  fn test_compact() {
    let mut arr = [1, 2, 3, 4, 5];
    let new_len = compact(&mut arr, |x| *x % 2 == 0);
    assert_eq!(new_len, 3);
    assert_eq!(&arr[..new_len], &[1, 3, 5]);

    let mut arr = [1, 2, 3, 4, 5];
    compact_goodrich(&mut arr, |x| *x % 2 == 0);
    assert_eq!(&arr[..3], &[1, 3, 5]);
  }

  #[test]
  fn test_small() {
    let mut arr: Vec<i32> = vec![1];
    let new_len = compact(&mut arr, |x| *x % 2 == 0);
    assert_eq!(new_len, 1);
    assert_eq!(&arr[..new_len], &[1]);
    let mut arr: Vec<i32> = vec![1];
    compact_goodrich(&mut arr, |x| *x % 2 == 0);
    assert_eq!(&arr[..1], &[1]);

    let mut arr: Vec<i32> = vec![2];
    let new_len = compact(&mut arr, |x| *x % 2 == 0);
    assert_eq!(new_len, 0);
    assert_eq!(&arr[..new_len], &[]);

    let mut arr: Vec<i32> = vec![1, 2];
    let new_len = compact(&mut arr, |x| *x % 2 == 0);
    assert_eq!(new_len, 1);
    assert_eq!(&arr[..new_len], &[1]);
    let mut arr: Vec<i32> = vec![1, 2];
    compact_goodrich(&mut arr, |x| *x % 2 == 0);
    assert_eq!(&arr[..1], &[1]);

    let mut arr: Vec<i32> = vec![];
    let new_len = compact(&mut arr, |x| *x % 2 == 0);
    assert_eq!(new_len, 0);
    assert_eq!(&arr[..new_len], &[]);
    let mut arr: Vec<i32> = vec![];
    compact_goodrich(&mut arr, |x| *x % 2 == 0);
    assert_eq!(&arr[..0], &[]);
  }

  #[test]
  fn test_many_sizes() {
    // Picks a random array size and fills with random values and checks if it's correct via a non oblivious comparison
    let mut rng = rand::rng();
    for _i in 0..100 {
      let size = rng.random_range(0..2050);
      let arr: Vec<i32> = (0..size).map(|_| rng.random_range(0..100)).collect();
      let mut arr1 = arr.clone();
      let new_len = compact(&mut arr1, |x| *x % 2 == 0);
      for itm in arr1.iter().take(new_len) {
        assert!(itm % 2 != 0);
      }
      for itm in arr1.iter().skip(new_len) {
        assert!(itm % 2 == 0);
      }
      let mut arr2 = arr.clone();
      compact_goodrich(&mut arr2, |x| *x % 2 == 0);
      for itm in arr2.iter().take(new_len) {
        assert!(itm % 2 != 0);
      }
      for itm in arr2.iter().skip(new_len) {
        assert!(itm % 2 == 0);
      }
    }
  }

  #[test]
  fn test_distribute() {
    let mut arr = [1, 3, 5, 0, 2, 4];
    let payload = [0, 1, 2, 3, 3, 4, 5];
    distribute_payload(&mut arr, &payload);
    assert_eq!(&arr, &[1, 3, 5, 4, 0, 2]);

    let mut arr = [1, 2, 3, 4, 5];
    let payload = [0, 1, 1, 2, 2, 3];
    compact_payload(&mut arr, &payload);
    assert_eq!(&arr[..3], &[1, 3, 5]);
    distribute_payload(&mut arr, &payload);
    assert_eq!(&arr, &[1, 2, 3, 4, 5]);
  }

  #[test]
  fn test_distribute_after_compact_rands() {
    let mut rng = rand::rng();
    for _i in 0..100 {
      let size = rng.random_range(0..2050);
      let arr: Vec<i32> = (0..size).map(|_| rng.random_range(0..100)).collect();
      let mut arr1 = arr.clone();
      let mut payload = vec![0; size + 1];
      for i in 0..size {
        let mut adder = 1usize;
        adder.cmov(&0, arr[i] % 2 == 0);
        payload[i + 1] = payload[i] + adder;
      }
      compact_payload(&mut arr1, &payload);
      distribute_payload(&mut arr1, &payload);
      assert_eq!(&arr1, &arr);
    }
  }
}
