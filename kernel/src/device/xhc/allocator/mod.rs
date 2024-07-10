mod external;

use core::{mem::size_of, ptr::slice_from_raw_parts_mut};
pub use external::Allocator;

pub trait Allocatable {
    unsafe fn allocate(&mut self, size: usize, align: usize, boundary: usize) -> Option<*mut u8>;
    unsafe fn free(&mut self, addr: u64, size: usize);

    fn alloc<T>(&mut self, size: usize, align: usize, boundary: usize) -> Result<&mut T, ()> {
        unsafe {
            Ok(&mut *self
                .allocate(size, align, boundary)
                .expect("Not Enough Memory")
                .cast::<T>())
        }
    }

    fn alloc_array<T>(
        &mut self,
        length: usize,
        align: usize,
        boundary: usize,
    ) -> Result<&'static mut [T], ()> {
        let buf = unsafe {
            self.allocate(size_of::<T>() * length, align, boundary)
                .expect("Not Enough Memory") as *mut T
        };
        Ok(unsafe { &mut *slice_from_raw_parts_mut(buf, length) })
    }
}
