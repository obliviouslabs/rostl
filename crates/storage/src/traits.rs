//! Traits related to storage.
use std::io;

use bytemuck::{Pod, Zeroable};

/// A trait for types that can be efficiently stored in block storage.
/// Requires the type to be `Pod` and `Zeroable`.
/// We don't use serialize/deserialize traits to minimize overhead.
/// (This type can return the raw bytes typed correctly.)
pub trait Storable: Pod + Zeroable {}
impl<T: Pod + Zeroable> Storable for T {}

/// A trait for reading and writing pages of a fixed sized.
pub trait PageStorage: Sized // Sized is needed to wrap in an io::Result
{
  /// The size of a page in bytes.
  const PAGE_SIZE: usize;

  /// Opens a page storage with the given key and size in bytes.
  /// Should create a new blob with `$\ceil{bytes / PAGE_SIZE}$` pages if the blob doesn't exist.
  /// Can fail if the blob exists and has a different size (up to the implementation).
  fn open(key: String, pages: usize) -> io::Result<Self>;

  /// Reads a page. Pages are 0-indexed and have size `PAGE_SIZE`.
  fn read_page(&self, page_idx: usize, ret: &mut [u8]) -> io::Result<()>;

  /// Writes a page. Pages are 0-indexed and have size `PAGE_SIZE`.
  fn write_page(&self, page_idx: usize, data: &[u8]) -> io::Result<()>;

  /// Returns the number of pages in the storage.
  fn pages_len(&self) -> usize;
}
