use super::Allocatable;

const MEMORY_POOL_SIZE: usize = 4096 * 32;

static mut MEMORY_POOL: [u8; MEMORY_POOL_SIZE] = [0u8; MEMORY_POOL_SIZE];
static mut ALLOC_PTR: *mut u8 = unsafe { &mut MEMORY_POOL[0] };

pub struct Allocator {
    current_addr: *mut u8,
    end_addr: *mut u8,
}

impl Allocator {
    pub fn new_with_addr(buff_addr: u64) -> Self {
        let current_addr = buff_addr as *mut u8;
        Self {
            current_addr,
            end_addr: unsafe { current_addr.add(MEMORY_POOL_SIZE) },
        }
    }

    pub fn new() -> Self {
        let current_addr = unsafe { &mut MEMORY_POOL[0] as *mut u8 };
        Self {
            current_addr,
            end_addr: unsafe { current_addr.add(MEMORY_POOL_SIZE) },
        }
    }
}

impl Allocatable for Allocator {
    unsafe fn allocate(&mut self, size: usize, align: usize, boundary: usize) -> Option<*mut u8> {
        if align > 0 {
            let offset = self.current_addr.align_offset(align);
            self.current_addr = self.current_addr.add(offset);
        }
        if boundary > 0 {
            let offset = self.current_addr.align_offset(boundary);
            let next_boundary = self.current_addr.add(offset);
            if (next_boundary as usize) < (self.current_addr as usize) + size {
                self.current_addr = next_boundary;
            }
        }
        if self.end_addr < self.current_addr.add(size) {
            None
        } else {
            let ptr = self.current_addr;
            self.current_addr = self.current_addr.add(size);
            Some(ptr)
        }
    }

    unsafe fn free(&mut self, addr: u64, size: usize) {}
}
