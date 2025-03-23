//! Prelude module for the ORAM crate
///
///
/// Type used for positions in the position map
pub type PositionType = u32;
/// The default invalid position
pub const DUMMY_POS: PositionType = PositionType::MAX;

// UNDONE(git-26): This should be a generic type across the crate to support safe map, not just usize
/// Type used for ORAM Indexes
pub type K = usize;
