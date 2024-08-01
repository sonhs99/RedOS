use super::{Allocator, FrameID, BYTE_PER_FRAME};
use crate::sync::{Mutex, OnceLock};
use core::{
    alloc::{GlobalAlloc, Layout},
    ptr,
};

pub struct DumpAllocator {
    ptr: u64,
    end_ptr: u64,
}

impl DumpAllocator {
    pub const fn new(start_addr: usize, heap_size: usize) -> Self {
        Self {
            ptr: start_addr as u64,
            end_ptr: (start_addr + heap_size) as u64,
        }
    }
}
impl Allocator for DumpAllocator {
    fn allocate(&mut self, layout: Layout) -> Result<*mut u8, ()> {
        let aligned_base = super::align(self.ptr, layout.align() as u64);
        let aligned_end = super::align(aligned_base + layout.size() as u64, layout.align() as u64);

        if aligned_end >= self.end_ptr {
            Err(())
        } else {
            self.ptr = aligned_end;
            Ok(aligned_base as *mut u8)
        }
    }

    fn free(&mut self, ptr: *mut u8, layout: Layout) {}
}
