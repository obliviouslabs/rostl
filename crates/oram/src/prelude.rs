/// Type used for positions in the position map
pub(crate) type PositionType = usize;
pub(crate) const DUMMY_POS: PositionType = PositionType::MAX;

// UNDONE(git-26): This should be a generic type across the crate to support safe map, not just usize
/// Type used for ORAM Indexes
pub(crate) type K = usize;

// pub(crate) const fn max(a: usize, b: usize) -> usize {
//   if a > b {
//     a
//   } else {
//     b
//   }
// }

pub(crate) const fn min(a: usize, b: usize) -> usize {
  if a < b {
    a
  } else {
    b
  }
}
