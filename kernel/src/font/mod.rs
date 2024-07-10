use core::include_bytes;

use crate::graphic::{GraphicWriter, PixelColor};

static VGA_FONT: &[u8; 4096] = include_bytes!("IBM_VGA_8x16.bin");

pub fn get_font(c: usize) -> Option<&'static [u8]> {
    if c < 256 {
        let index = c * 16;
        // Some(&ASCII_FONT[index..(index + 16)])
        Some(&VGA_FONT[index..(index + 16)])
    } else {
        None
    }
}

pub fn write_ascii(writer: &GraphicWriter, x: u64, y: u64, c: u8, color: PixelColor) {
    if let Some(font) = get_font(c as usize) {
        for dy in 0..16usize {
            for dx in 0..8usize {
                if (font[dy] << dx) & 0x80 != 0 {
                    writer.write(x as usize + dx, y as usize + dy, color)
                }
            }
        }
    };
}
