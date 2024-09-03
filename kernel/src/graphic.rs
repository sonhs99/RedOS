use bootloader::{FrameBufferConfig, PixelFormat};
use static_box::Box;

use core::ptr::{copy_nonoverlapping, slice_from_raw_parts_mut};

use crate::sync::{Mutex, MutexGuard, OnceLock, StaticCell};

pub static PIXEL_WRITER: OnceLock<Mutex<GraphicWriter>> = OnceLock::new();
static FORMAT: OnceLock<PixelFormat> = OnceLock::new();

#[derive(Clone, Copy, PartialEq)]
pub struct PixelColor(u8, u8, u8);

impl PixelColor {
    pub const Black: PixelColor = PixelColor(0, 0, 0);
    pub const White: PixelColor = PixelColor(255, 255, 255);
    pub const Red: PixelColor = PixelColor(255, 0, 0);
    pub const Green: PixelColor = PixelColor(0, 255, 0);
    pub const Blue: PixelColor = PixelColor(0, 0, 255);

    pub fn as_rgbx(&self) -> u32 {
        let red = self.0 as u32;
        let green = self.1 as u32;
        let blue = self.2 as u32;
        blue << 16 | green << 8 | red
    }

    pub fn as_bgrx(&self) -> u32 {
        let red = self.0 as u32;
        let green = self.1 as u32;
        let blue = self.2 as u32;
        red << 16 | green << 8 | blue
    }

    pub fn as_u32(&self) -> u32 {
        match FORMAT.get().expect("Unknown Pixel Format") {
            PixelFormat::RGBReserved8 => self.as_rgbx(),
            PixelFormat::BGRReserved8 => self.as_bgrx(),
            _ => panic!("Not Support for This Format"),
        }
    }
}

impl From<u32> for PixelColor {
    fn from(value: u32) -> Self {
        let (r, g, b) = match FORMAT.get().expect("Unknown Pixel Format") {
            PixelFormat::RGBReserved8 => ((value >> 16) as u8, (value >> 8) as u8, value as u8),
            PixelFormat::BGRReserved8 => (value as u8, (value >> 8) as u8, (value >> 16) as u8),
            _ => panic!("Not Support for This Format"),
        };
        PixelColor(r, g, b)
    }
}

pub struct GraphicWriter {
    pub(crate) frame_config: FrameBufferConfig,
    write_fn: fn(&GraphicWriter, usize, usize, PixelColor),
}

impl GraphicWriter {
    pub fn new(frame_config: FrameBufferConfig) -> Self {
        let write_fn = match FORMAT.get_or_init(|| frame_config.pixel_format()) {
            PixelFormat::RGBReserved8 => rgb_write,
            PixelFormat::BGRReserved8 => bgr_write,
            _ => panic!("Not Support for This Format"),
        };
        Self {
            frame_config,
            write_fn,
        }
    }

    fn pixel(&self, x: usize, y: usize) -> &mut u32 {
        let position = x + self.frame_config.stride() * y;
        let ptr = unsafe { (self.frame_config.address() as *mut u32).add(position) };
        unsafe { &mut *ptr }
    }

    pub fn write(&self, x: usize, y: usize, color: PixelColor) {
        let (width, height) = self.frame_config.resolution();
        if x < width || y < height {
            (self.write_fn)(self, x, y, color)
        }
    }

    pub fn write_buf(&self, x: usize, y: usize, buffer: &[u32]) {
        let (width, height) = self.frame_config.resolution();
        if y >= height {
            return;
        }
        let len = if buffer.len() > width - x {
            width - x
        } else {
            buffer.len()
        };
        let position = x + self.frame_config.stride() * y;
        let ptr = unsafe { (self.frame_config.address() as *mut u32).add(position) };
        let slice = unsafe { &mut *slice_from_raw_parts_mut(ptr, len) };
        for (src, dest) in buffer.iter().zip(slice.iter_mut()) {
            *dest = *src;
        }
    }

    pub fn clean(&self) {
        for x in 0..self.frame_config.resolution().0 {
            for y in 0..self.frame_config.resolution().1 {
                self.write(x, y, PixelColor::Black);
            }
        }
    }
}

fn bgr_write(writer: &GraphicWriter, x: usize, y: usize, color: PixelColor) {
    let pixel = writer.pixel(x, y);
    *pixel = color.as_bgrx();
}

fn rgb_write(writer: &GraphicWriter, x: usize, y: usize, color: PixelColor) {
    let pixel = writer.pixel(x, y);
    *pixel = color.as_rgbx();
}

pub fn init_graphic(frame_config: FrameBufferConfig) {
    PIXEL_WRITER.get_or_init(|| Mutex::new(GraphicWriter::new(frame_config)));
}

pub fn get_graphic() -> &'static Mutex<GraphicWriter> {
    PIXEL_WRITER.get().unwrap()
}
