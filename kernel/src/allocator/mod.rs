use crate::{
    println,
    sync::{Mutex, OnceLock},
};
use bootloader::MemoryMap;
use core::{
    alloc::{GlobalAlloc, Layout},
    ptr,
};
use log::debug;

pub mod dump;
pub mod frame;
pub mod slab;

const KiB: u64 = 1024;
const MiB: u64 = KiB * 1024;
const GiB: u64 = MiB * 1024;

const BYTE_PER_FRAME: u64 = 4 * KiB;

const MAX_PHYSICAL_MEMORY_BYTES: u64 = 128 * GiB;
const FRAME_COUNT: u64 = MAX_PHYSICAL_MEMORY_BYTES / BYTE_PER_FRAME;
const HEAP_FRAME_COUNT: usize = 64 * 512;

#[derive(Clone, Copy)]
struct FrameID(u64);

impl FrameID {
    pub const fn new(id: u64) -> Self {
        Self(id)
    }

    pub const fn id(&self) -> u64 {
        self.0
    }

    pub const fn frame(&self) -> u64 {
        self.0 * BYTE_PER_FRAME
    }
}

const NULL_FRAME: FrameID = FrameID::new(u64::MAX);

pub trait Allocator {
    fn allocate(&mut self, layout: Layout) -> Result<*mut u8, ()>;
    fn free(&mut self, ptr: *mut u8, layout: Layout);
}

#[global_allocator]
static ALLOCATOR: OnceLock<Mutex<slab::SlabAllocator>> = OnceLock::new();

static FRAME_MANAGER: OnceLock<Mutex<frame::FrameBitmapManager>> = OnceLock::new();

pub fn init_heap(memory_map: &MemoryMap) {
    FRAME_MANAGER.get_or_init(|| Mutex::new(frame::FrameBitmapManager::new()));
    FRAME_MANAGER.lock().scan(memory_map);
    FRAME_MANAGER.lock().mark_alloc(FrameID(1), 0x800);
    let start = FRAME_MANAGER.lock().allocate(HEAP_FRAME_COUNT).unwrap();
    let end = FrameID(start.id() + HEAP_FRAME_COUNT as u64);
    let start_addr = (start.id() * BYTE_PER_FRAME) as usize;
    let end_addr = (end.id() * BYTE_PER_FRAME) as usize;
    debug!("{start_addr:#X} - {end_addr:#X}");
    ALLOCATOR.get_or_init(|| unsafe {
        Mutex::new(slab::SlabAllocator::new(start_addr, end_addr - start_addr))
    });
}

pub fn malloc(size: usize, align: usize) -> *mut u8 {
    unsafe { ALLOCATOR.alloc(Layout::from_size_align(size, align).unwrap()) }
}

fn align(base: u64, align: u64) -> u64 {
    if align == 0 {
        base
    } else {
        let align_offset = base % align;
        let align_offset = align - align_offset;
        base + align_offset
    }
}

unsafe impl<A: Allocator> GlobalAlloc for OnceLock<Mutex<A>> {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        self.lock()
            .allocate(layout)
            .unwrap_or(ptr::null_mut() as *mut u8)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        self.lock().free(ptr, layout)
    }
}
