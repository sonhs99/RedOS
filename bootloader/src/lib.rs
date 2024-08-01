#![no_main]
#![no_std]
#![feature(core_intrinsics)]

use acpi::RSDP;

pub mod acpi;

#[derive(Clone, Copy, Debug)]
pub enum PixelFormat {
    RGBReserved8,
    BGRReserved8,
    Bitmask,
    BltOnly,
}

#[derive(Clone)]
pub struct FrameBufferConfig {
    height: usize,
    width: usize,
    pixel_per_scanline: usize,
    buffer_addr: u64,
    pixel_format: PixelFormat,
}

impl FrameBufferConfig {
    pub const fn new(
        height: usize,
        width: usize,
        pixel_per_scanline: usize,
        buffer_addr: u64,
        pixel_format: PixelFormat,
    ) -> Self {
        Self {
            height,
            width,
            pixel_per_scanline,
            buffer_addr,
            pixel_format,
        }
    }

    pub fn resolution(&self) -> (usize, usize) {
        (self.height, self.width)
    }

    pub fn address(&self) -> u64 {
        self.buffer_addr
    }

    pub fn stride(&self) -> usize {
        self.pixel_per_scanline
    }

    pub fn pixel_format(&self) -> PixelFormat {
        self.pixel_format
    }

    pub fn size(&self) -> usize {
        self.height * self.pixel_per_scanline * 4
    }
}

#[derive(Clone)]
pub struct MemoryMap {
    pub buffer_size: u64,
    pub buffer: *mut u8,
    pub map_size: u64,
    pub descriptor_size: u64,
}

impl MemoryMap {
    pub fn entries(&self) -> MemoryMapIter<'_> {
        MemoryMapIter {
            memory_map: self,
            index: 0,
        }
    }

    const fn len(&self) -> usize {
        (self.map_size / self.descriptor_size) as usize
    }

    pub fn get(&self, index: usize) -> Option<&MemoryDescriptor> {
        if index >= self.len() || index >= 110 {
            return None;
        }
        let descriptor = unsafe {
            &*self
                .buffer
                .add(self.descriptor_size as usize * index)
                .cast::<MemoryDescriptor>()
        };
        if descriptor.type_ as u32 >= MemoryType::MaxMemoryType as u32 {
            return None;
        }
        Some(descriptor)
    }
}

pub struct MemoryMapIter<'a> {
    memory_map: &'a MemoryMap,
    index: usize,
}

impl<'a> Iterator for MemoryMapIter<'a> {
    type Item = &'a MemoryDescriptor;

    fn next(&mut self) -> Option<Self::Item> {
        let descriptor = self.memory_map.get(self.index)?;
        self.index += 1;
        Some(descriptor)
    }
}

#[repr(C)]
pub struct MemoryDescriptor {
    pub type_: MemoryType,
    pub physical_start: u64,
    pub virtual_start: u64,
    pub number_of_pages: u64,
    pub attribute: u64,
}

#[repr(u32)]
#[derive(Debug, Clone, Copy)]
pub enum MemoryType {
    ReservedMemoryType = 0,
    LoaderCode,
    LoaderData,
    BootServicesCode,
    BootServicesData,
    RuntimeServicesCode,
    RuntimeServicesData,
    ConventionalMemory,
    UnusableMemory,
    ACPIReclaimMemory,
    ACPIMemoryNVS,
    MemoryMappedIO,
    MemoryMappedIOPortSpace,
    PalCode,
    PersistentMemory,
    MaxMemoryType,
}

#[derive(Clone)]
pub struct BootInfo {
    pub frame_config: FrameBufferConfig,
    pub memory_map: MemoryMap,
    pub rsdp: &'static RSDP,
}
