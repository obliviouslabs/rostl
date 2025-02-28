//! In-memory storage implementation.
use crate::traits::PageStorage;
use std::{io, sync::RwLock};

/// A simple in-memory storage implementation.
#[derive(Debug)]
pub struct MemStore {
  data: RwLock<Vec<u8>>,
}

impl PageStorage for MemStore {
  const PAGE_SIZE: usize = 4096;
  fn open(_key: String, total_pages: usize) -> io::Result<Self> {
    Ok(Self { data: RwLock::new(vec![0; total_pages * Self::PAGE_SIZE]) })
  }

  fn read_page(&self, block_idx: usize, buf: &mut [u8]) -> io::Result<()> {
    let start = block_idx * Self::PAGE_SIZE;
    let end = start + Self::PAGE_SIZE;
    // UNDONE(git-22): Avoid double copy.
    buf.copy_from_slice(&self.data.read().unwrap()[start..end]);
    Ok(())
  }

  fn write_page(&self, block_idx: usize, buf: &[u8]) -> io::Result<()> {
    let start = block_idx * Self::PAGE_SIZE;
    let end = start + Self::PAGE_SIZE;
    self.data.write().unwrap()[start..end].copy_from_slice(buf);
    Ok(())
  }

  fn pages_len(&self) -> usize {
    self.data.read().unwrap().len() / Self::PAGE_SIZE
  }
}
