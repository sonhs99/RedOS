use bootloader::{FrameBufferConfig, PixelFormat};

use core::ptr::slice_from_raw_parts_mut;

use crate::sync::{Mutex, OnceLock};

pub static PIXEL_WRITER: OnceLock<Mutex<GraphicWriter>> = OnceLock::new();

#[derive(Clone, Copy, PartialEq)]
pub struct PixelColor(u8, u8, u8);

impl PixelColor {
    pub const Black: PixelColor = PixelColor(0, 0, 0);
    pub const White: PixelColor = PixelColor(255, 255, 255);
    pub const Red: PixelColor = PixelColor(255, 0, 0);
    pub const Green: PixelColor = PixelColor(0, 255, 0);
    pub const Blue: PixelColor = PixelColor(0, 0, 255);
}

pub struct GraphicWriter {
    pub(crate) frame_config: FrameBufferConfig,
    write_fn: fn(&GraphicWriter, usize, usize, PixelColor),
}

impl GraphicWriter {
    pub fn new(frame_config: FrameBufferConfig) -> Self {
        let write_fn = match frame_config.pixel_format() {
            PixelFormat::RGBReserved8 => rgb_write,
            PixelFormat::BGRReserved8 => bgr_write,
            _ => panic!(),
        };
        Self {
            frame_config,
            write_fn,
        }
    }

    pub fn pixel(&self, x: usize, y: usize) -> &'static mut [u8] {
        let position: usize = x + self.frame_config.stride() * y;
        let address = self.frame_config.address() + position as u64 * 4;
        unsafe { &mut *slice_from_raw_parts_mut(address as *mut u8, 4) }
    }

    pub fn write(&self, x: usize, y: usize, color: PixelColor) {
        (self.write_fn)(self, x, y, color)
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
    pixel[0] = color.2;
    pixel[1] = color.1;
    pixel[2] = color.0;
}

fn rgb_write(writer: &GraphicWriter, x: usize, y: usize, color: PixelColor) {
    let pixel = writer.pixel(x, y);
    pixel[0] = color.0;
    pixel[1] = color.1;
    pixel[2] = color.2;
}

pub fn init_graphic(frame_config: FrameBufferConfig) {
    PIXEL_WRITER.get_or_init(|| Mutex::new(GraphicWriter::new(frame_config)));
}

pub fn get_graphic() -> &'static Mutex<GraphicWriter> {
    PIXEL_WRITER.get().unwrap()
}

unsafe impl Send for GraphicWriter {}
unsafe impl Sync for GraphicWriter {}
