use alloc::vec;

use crate::{font::write_ascii, graphic::PixelColor};

use super::{Area, Drawable, Writable};

pub struct Rectangle {
    width: usize,
    height: usize,
    border: usize,
    margin: usize,
    border_color: PixelColor,
    background_color: PixelColor,
    foreground_color: PixelColor,
}

impl Rectangle {
    pub fn new(
        width: usize,
        height: usize,
        border: usize,
        margin: usize,
        border_color: PixelColor,
        background_color: PixelColor,
        foreground_color: PixelColor,
    ) -> Self {
        Self {
            width,
            height,
            border,
            margin,
            border_color,
            background_color,
            foreground_color,
        }
    }

    pub fn inside_pos(&self, offset_x: usize, offset_y: usize) -> Area {
        Area::new(
            self.border + self.margin + offset_x,
            self.border + self.margin + offset_y,
            self.width,
            self.height,
        )
    }

    pub fn outside_pos(&self, offset_x: usize, offset_y: usize) -> Area {
        Area::new(
            offset_x,
            offset_y,
            self.width + self.border + self.margin,
            self.height + self.border + self.margin,
        )
    }
}

impl Drawable for Rectangle {
    fn draw(&self, offset_x: usize, offset_y: usize, area: &Area, writer: &mut impl Writable) {
        let width_len = self.width + (self.border + self.margin) * 2;
        let height_len = self.height + (self.border + self.margin) * 2;

        let border_hor = vec![self.border_color.as_u32(); width_len];
        let bg = vec![self.background_color.as_u32(); width_len - self.border * 2];
        let border_ver = vec![self.border_color.as_u32(); self.border];
        for idx_y in 0..=height_len {
            if idx_y < self.border || idx_y > height_len - self.border {
                writer.write_buf(offset_x, offset_y + idx_y, &border_hor);
            } else {
                writer.write_buf(offset_x, offset_y + idx_y, &border_ver);
                writer.write_buf(offset_x + self.border, offset_y + idx_y, &bg);
                writer.write_buf(
                    offset_x + width_len - self.border,
                    offset_y + idx_y,
                    &border_ver,
                );
            }
        }
    }
}

// pub struct Button {
//     width: usize,
//     height: usize,
//     border: usize,
// }

pub fn write_str(
    offset_x: usize,
    offset_y: usize,
    str: &str,
    foreground: PixelColor,
    background: PixelColor,
    writer: &mut impl Writable,
) {
    let mut x = 0usize;
    let mut y = 0usize;
    for (idx, c) in str.bytes().enumerate() {
        if c >= 0x20 && c <= 0x7F {
            write_ascii(
                (x * 8 + offset_x) as u64,
                (y * 16 + offset_y) as u64,
                c,
                foreground,
                background,
                writer,
            );
            x += 1;
        } else if c == b'\n' {
            y += 1;
            x = 0;
        }
    }
}
