use core::alloc::Layout;

use log::debug;

use crate::task::Task;

use super::{dump::DumpAllocator, Allocator};

const NUM_OF_SLAB: usize = 8;

struct Slab {
    block_size: usize,
    free_block: BlockList,
}

impl Slab {
    pub unsafe fn new(start_addr: usize, slab_size: usize, block_size: usize) -> Self {
        let num_of_block = slab_size / block_size;
        Self {
            block_size,
            free_block: BlockList::new(start_addr, block_size, num_of_block),
        }
    }

    pub fn block_size(&self) -> usize {
        self.block_size
    }

    pub unsafe fn grow(&mut self, start_addr: usize, slab_size: usize) {
        let num_of_block = slab_size / self.block_size;
        let mut new_list = BlockList::new(start_addr, self.block_size, num_of_block);
        while let Some(block) = new_list.pop() {
            self.free_block.push(block);
        }
    }

    pub fn allocate(&mut self, layout: Layout) -> Result<*mut u8, ()> {
        match self.free_block.pop() {
            Some(block) => Ok(block.addr() as *mut u8),
            None => Err(()),
        }
    }

    pub fn free(&mut self, ptr: *mut u8) {
        let block = ptr.cast::<Block>();
        unsafe { self.free_block.push(&mut *block) };
    }
}

struct BlockList {
    len: usize,
    head: Option<&'static mut Block>,
}

impl BlockList {
    pub unsafe fn new(start_addr: usize, block_size: usize, num_of_block: usize) -> Self {
        let mut list = Self::empty();
        for block_idx in (0..num_of_block).rev() {
            let block = (start_addr + block_idx * block_size) as *mut Block;
            // if block_idx % 0x100 == 0 {
            //     debug!("{block_size}: {block_idx:#X}/{num_of_block:#X} = {block:?}");
            // }
            list.push(&mut *block);
        }
        list
    }

    fn empty() -> Self {
        Self { len: 0, head: None }
    }

    fn push(&mut self, block: &'static mut Block) {
        block.next = self.head.take();
        self.len += 1;
        self.head = Some(block);
    }

    fn pop(&mut self) -> Option<&'static mut Block> {
        self.head.take().map(|block| {
            self.head = block.next.take();
            self.len -= 1;
            block
        })
    }

    pub fn is_empty(&self) -> bool {
        self.head.is_none()
    }
}

struct Block {
    next: Option<&'static mut Block>,
}

impl Block {
    pub fn addr(&self) -> usize {
        self as *const _ as usize
    }
}

pub struct SlabAllocator {
    slab: [Slab; NUM_OF_SLAB - 1],
    fallback: DumpAllocator,
}

impl SlabAllocator {
    pub unsafe fn new(start_addr: usize, heap_size: usize) -> Self {
        let slab_size = heap_size / NUM_OF_SLAB;
        let slab = [
            Slab::new(start_addr + 0 * slab_size, slab_size, 64),
            Slab::new(start_addr + 1 * slab_size, slab_size, 128),
            Slab::new(start_addr + 3 * slab_size, slab_size, 256),
            Slab::new(start_addr + 4 * slab_size, slab_size, 512),
            Slab::new(start_addr + 5 * slab_size, slab_size, 1024),
            Slab::new(start_addr + 6 * slab_size, slab_size, 2048),
            Slab::new(start_addr + 7 * slab_size, slab_size, 4096),
        ];
        let fallback = DumpAllocator::new(start_addr + 8 * slab_size, slab_size);
        Self { slab, fallback }
    }

    pub fn get_size(&self, layout: Layout) -> Option<usize> {
        self.slab.iter().enumerate().find_map(|(idx, slab)| {
            let block_size = slab.block_size;
            if layout.size() <= block_size && layout.align() <= block_size {
                Some(idx)
            } else {
                None
            }
        })
    }

    pub unsafe fn grow(&mut self, mem_start_addr: usize, mem_size: usize, slab: Option<usize>) {
        if let Some(slab_idx) = slab {
            self.slab[slab_idx].grow(mem_start_addr, mem_size);
        }
    }
}

impl Allocator for SlabAllocator {
    fn allocate(&mut self, layout: Layout) -> Result<*mut u8, ()> {
        match self.get_size(layout) {
            Some(slab_idx) => self.slab[slab_idx].allocate(layout),
            None => self.fallback.allocate(layout),
        }
    }

    fn free(&mut self, ptr: *mut u8, layout: Layout) {
        match self.get_size(layout) {
            Some(slab_idx) => self.slab[slab_idx].free(ptr),
            None => self.fallback.free(ptr, layout),
        }
    }
}
