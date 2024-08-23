use core::{alloc::Layout, mem::size_of};

use bootloader::{MemoryDescriptor, MemoryMap, MemoryType};
use log::debug;

use super::{Allocator, FrameID, BYTE_PER_FRAME, FRAME_COUNT, FRAME_MANAGER};
use crate::page::UEFI_PAGE_SIZE;

type BitmapType = u8;

#[derive(Clone, Copy)]
struct Bitmap(BitmapType);

impl Bitmap {
    pub const fn new() -> Self {
        Self(0)
    }

    pub const fn bits() -> usize {
        size_of::<BitmapType>() * 8
    }

    pub const fn get(&self, idx: u8) -> Result<bool, ()> {
        if idx >= Self::bits() as u8 {
            Err(())
        } else {
            Ok((self.0 >> idx as BitmapType) & 0x01 != 0)
        }
    }

    pub fn set(&mut self, idx: u8, bit: bool) -> Result<(), ()> {
        if idx >= Self::bits() as u8 {
            Err(())
        } else {
            let value = 0x01 << idx as BitmapType;
            if bit {
                self.0 = self.0 | value;
            } else {
                self.0 = self.0 & !value;
            }
            Ok(())
        }
    }
}

pub struct FrameBitmapManager {
    bitmap: [Bitmap; FRAME_COUNT as usize / Bitmap::bits()],
    range_begin: FrameID,
    range_end: FrameID,
}

impl FrameBitmapManager {
    pub fn new() -> Self {
        Self {
            bitmap: [Bitmap::new(); FRAME_COUNT as usize / Bitmap::bits()],
            range_begin: FrameID(0),
            range_end: super::NULL_FRAME,
        }
    }

    pub fn scan(&mut self, memory_map: &MemoryMap) {
        let mut avail_end = 0;

        for desc in memory_map.entries() {
            if avail_end < desc.physical_start {
                self.mark_alloc(
                    FrameID(avail_end / BYTE_PER_FRAME),
                    ((desc.physical_start - avail_end) / BYTE_PER_FRAME) as usize,
                );
            }
            let physical_end = desc.physical_start + desc.number_of_pages * UEFI_PAGE_SIZE as u64;
            // debug!("{:?}: {}", desc.type_, is_available(desc));
            if is_available(desc) {
                avail_end = physical_end;
            } else {
                // debug!(
                //     "mark: {:#X} - {:#X}",
                //     desc.physical_start,
                //     desc.number_of_pages as usize * UEFI_PAGE_SIZE
                // );
                self.mark_alloc(
                    FrameID(desc.physical_start / BYTE_PER_FRAME),
                    (desc.number_of_pages as usize * UEFI_PAGE_SIZE) / BYTE_PER_FRAME as usize,
                );
            }
        }

        self.set_range(FrameID(1), FrameID(avail_end / BYTE_PER_FRAME));
    }

    pub fn mark_alloc(&mut self, begin: FrameID, size: usize) {
        for frame in begin.id()..begin.id() + size as u64 {
            self.set_bitmap(FrameID(frame), true).unwrap();
        }
    }

    pub fn set_range(&mut self, begin: FrameID, end: FrameID) {
        self.range_begin = begin;
        self.range_end = end;
    }

    pub fn allocate(&mut self, size: usize) -> Result<FrameID, ()> {
        let mut count = 0u64;
        let mut base = self.range_begin.id();
        while base + count <= self.range_end.id() {
            if count == size as u64 {
                let frame = FrameID(base);
                self.mark_alloc(frame, size);
                return Ok(frame);
            }
            if self.get_bitmap(FrameID(base + count))? {
                base += count + 1;
                count = 0;
            } else {
                // debug!("{base} -> {count}");
                count += 1;
            }
        }
        Err(())
    }

    pub fn free(&mut self, begin: FrameID, size: usize) -> Result<(), ()> {
        for frame in begin.id()..begin.id() + size as u64 {
            self.set_bitmap(FrameID(frame), false);
        }
        Ok(())
    }

    fn set_bitmap(&mut self, frame: FrameID, bit: bool) -> Result<(), ()> {
        let bitmap_idx = frame.id() as usize / Bitmap::bits();
        let bit_idx = frame.id() as usize % Bitmap::bits();
        self.bitmap[bitmap_idx].set(bit_idx as u8, bit)
    }

    fn get_bitmap(&mut self, frame: FrameID) -> Result<bool, ()> {
        let bitmap_idx = frame.id() as usize / Bitmap::bits();
        let bit_idx = frame.id() as usize % Bitmap::bits();
        self.bitmap[bitmap_idx].get(bit_idx as u8)
    }
}

pub struct FrameAllocator {}

impl FrameAllocator {
    pub const fn new() -> Self {
        Self {}
    }
}

impl Allocator for FrameAllocator {
    fn allocate(&mut self, layout: Layout) -> Result<*mut u8, ()> {
        let frame_size = (layout.size() + BYTE_PER_FRAME as usize - 1) / BYTE_PER_FRAME as usize;
        let ptr = FRAME_MANAGER.lock().allocate(frame_size)?;
        Ok((ptr.0 * BYTE_PER_FRAME) as *mut u8)
    }

    fn free(&mut self, ptr: *mut u8, layout: Layout) {
        let ptr = ptr as u64;
        let frame = FrameID::new(ptr / BYTE_PER_FRAME);
        let frame_size = (layout.size() + BYTE_PER_FRAME as usize - 1) / BYTE_PER_FRAME as usize;
        FRAME_MANAGER.lock().free(frame, frame_size);
    }
}

pub fn is_available(descriptor: &MemoryDescriptor) -> bool {
    match descriptor.type_ {
        MemoryType::BootServicesCode
        | MemoryType::BootServicesData
        | MemoryType::ConventionalMemory => true,
        _ => false,
    }
}
