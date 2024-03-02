#![no_main]
#![no_std]

#[derive(Clone, Copy, Debug)]
pub enum PixelFormat {
    RGBReserved8,
    BGRReserved8,
    Bitmask,
    BltOnly,
}

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

pub struct BootInfo {
    pub frame_config: FrameBufferConfig,
}
