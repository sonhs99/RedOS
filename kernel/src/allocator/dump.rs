use super::{FrameID, BYTE_PER_FRAME};
use crate::sync::{Mutex, OnceLock};
use core::{
    alloc::{GlobalAlloc, Layout},
    ptr,
};

pub struct DumpAllocator {
    start_frame: FrameID,
    end_frame: FrameID,
    ptr: u64,
    end_ptr: u64,
}

impl DumpAllocator {
    pub const fn new(start_frame: FrameID, end_frame: FrameID) -> Self {
        Self {
            start_frame,
            end_frame,
            ptr: start_frame.id() * BYTE_PER_FRAME,
            end_ptr: end_frame.id() * BYTE_PER_FRAME,
        }
    }
}

unsafe impl GlobalAlloc for OnceLock<Mutex<DumpAllocator>> {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let mut alloc = self.lock();

        let aligned_base = super::align(alloc.ptr, layout.align() as u64);
        let aligned_end = super::align(aligned_base + layout.size() as u64, layout.align() as u64);

        if aligned_end >= alloc.end_ptr {
            ptr::null_mut() as *mut u8
        } else {
            alloc.ptr = aligned_end;
            aligned_base as *mut u8
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {}
}
