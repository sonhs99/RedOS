use crate::{
    println,
    sync::{Mutex, OnceLock},
};
use bootloader::MemoryMap;
use core::alloc::{GlobalAlloc, Layout};
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
// const HEAP_FRAME_COUNT: usize = 1;

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

#[global_allocator]
static ALLOCATOR: OnceLock<Mutex<dump::DumpAllocator>> = OnceLock::new();

static FRAME_MANAGER: OnceLock<Mutex<frame::FrameBitmapManager>> = OnceLock::new();

pub fn init_heap(memory_map: &MemoryMap) {
    let manager = FRAME_MANAGER.get_or_init(|| Mutex::new(frame::FrameBitmapManager::new()));
    manager.lock().scan(memory_map);
    let start = manager.lock().allocate(HEAP_FRAME_COUNT).unwrap();
    let end = FrameID(start.id() + HEAP_FRAME_COUNT as u64);
    ALLOCATOR.get_or_init(|| Mutex::new(dump::DumpAllocator::new(start, end)));
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
