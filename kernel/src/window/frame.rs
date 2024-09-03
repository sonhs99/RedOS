use alloc::vec;

use crate::{font::write_ascii, graphic::PixelColor};

use super::{new_layer, new_layer_pos, Movable, WindowWriter, Writable};

pub struct WindowFrame {
    writer: WindowWriter,
    width: usize,
    height: usize,
}

impl WindowFrame {
    pub fn new_pos(x: usize, y: usize, width: usize, height: usize, name: &str) -> WindowFrame {
        let mut writer = new_layer_pos(x, y, width + 2, height + 19);
        Self::inner_new(writer, width, height, name)
    }
    pub fn new(width: usize, height: usize, name: &str) -> WindowFrame {
        let mut writer = new_layer(width + 2, height + 19);
        Self::inner_new(writer, width, height, name)
    }

    fn inner_new(mut writer: WindowWriter, width: usize, height: usize, name: &str) -> Self {
        let line_buffer = vec![PixelColor::White.as_u32(); width + 2];
        let bg_buffer = vec![PixelColor::Black.as_u32(); width];
        for idx_y in 0..height + 19 {
            if idx_y == 0 || idx_y == 17 || idx_y == height + 18 {
                writer.write_buf(0, idx_y, &line_buffer);
            } else {
                writer.write(0, idx_y, PixelColor::White);
                writer.write_buf(1, idx_y, &bg_buffer);
                writer.write(width + 1, idx_y, PixelColor::White);
            }
        }
        for (idx, &c) in name.as_bytes().iter().enumerate() {
            if c >= 0x20 && c <= 0x7F {
                write_ascii(
                    (idx * 8 + 1) as u64,
                    1,
                    c,
                    PixelColor::White,
                    PixelColor::Black,
                    &mut writer,
                );
            }
        }
        Self {
            writer,
            width,
            height,
        }
    }
}

impl Writable for WindowFrame {
    fn write(&mut self, x: usize, y: usize, color: PixelColor) {
        self.writer.write(x + 1, y + 18, color);
    }

    fn write_buf(&mut self, offset_x: usize, offset_y: usize, buffer: &[u32]) {
        self.writer.write_buf(offset_x + 1, offset_y + 18, buffer);
    }
}

impl Movable for WindowFrame {
    fn move_(&mut self, offset_x: isize, offset_y: isize) {
        self.writer.move_range(
            offset_x,
            offset_y,
            super::Rectangle {
                x: 1,
                y: 18,
                width: self.width,
                height: self.height,
            },
        );
        // self.writer.move_(offset_x, offset_y)
    }

    fn move_range(&mut self, offset_x: isize, offset_y: isize, area: super::Rectangle) {
        self.writer.move_range(offset_x, offset_y, area);
    }
}
