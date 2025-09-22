//! Some generic utils that should probably be in another module

/// Syntactic sugar for `arr.cswap(i,i,a[i]>a[j])`
#[macro_export]
macro_rules! CSWAP {
  ($arr:expr, $i:expr, $j:expr) => {
    $arr.cswap($i, $j, $arr[$i] > $arr[$j]);
  };
}
