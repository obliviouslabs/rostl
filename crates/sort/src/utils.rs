//! Some generic utils that should probably be in another module

/// Returns the smallest power of two that is strictly greater than the given size.
#[inline]
pub const fn get_strictly_bigger_power_of_two(size: usize) -> usize {
  // Using this hand-optimized version is slower:
  // 1 << (usize::BITS - size.leading_zeros()) as usize
  //
  let mut n = 1;
  while n <= size {
    n *= 2;
  }
  n
}


/// Syntatic sugar for arr.cswap(i,i,a[i]>a[j])
#[macro_export]
macro_rules! CSWAP {
  ($arr:expr, $i:expr, $j:expr) => {
    $arr.cswap($i, $j, $arr[$i] > $arr[$j]);
  };
}
