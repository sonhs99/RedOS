use alloc::vec;

use crate::{font::write_ascii, graphic::PixelColor};

use super::{
    component::{write_str, Rectangle},
    create_window, create_window_pos,
    event::Event,
    Area, Drawable, Movable, PartialWriter, WindowWriter, Writable,
};

const CLOSE_BUTTON_IMG: [[u8; 8]; 4] = [
    [
        0b1100_0000,
        0b1110_0000,
        0b0111_0000,
        0b0011_1000,
        0b0001_1100,
        0b0000_1110,
        0b0000_0110,
        0b0000_0000,
    ],
    [
        0b0000_0011,
        0b0000_0111,
        0b0000_1110,
        0b0001_1100,
        0b0011_1000,
        0b0111_0000,
        0b0110_0000,
        0b0000_0000,
    ],
    [
        0b0000_0011,
        0b0000_0111,
        0b0000_1110,
        0b0001_1100,
        0b0011_1000,
        0b0111_0000,
        0b0110_0000,
        0b0000_0000,
    ],
    [
        0b1100_0000,
        0b1110_0000,
        0b0111_0000,
        0b0011_1000,
        0b0001_1100,
        0b0000_1110,
        0b0000_0110,
        0b0000_0000,
    ],
];

pub struct WindowFrame {
    writer: WindowWriter,
    width: usize,
    height: usize,
    title: Rectangle,
    info: Rectangle,
}

impl WindowFrame {
    pub fn new_pos(x: usize, y: usize, width: usize, height: usize, name: &str) -> WindowFrame {
        let mut writer = create_window_pos(x, y, width + 6, height + 24);
        Self::inner_new(writer, width, height, name)
    }
    pub fn new(width: usize, height: usize, name: &str) -> WindowFrame {
        let mut writer = create_window(width + 6, height + 24);
        Self::inner_new(writer, width, height, name)
    }

    fn inner_new(mut writer: WindowWriter, width: usize, height: usize, name: &str) -> Self {
        let line_buffer = vec![PixelColor::White.as_u32(); width + 4];
        let bg_buffer = vec![PixelColor::Black.as_u32(); width + 2];
        let border = Rectangle::new(
            width + 4,
            height + 22,
            1,
            0,
            PixelColor::White,
            PixelColor::White,
            PixelColor::Black,
        );
        border.draw(0, 0, &border.inside_pos(0, 0), &mut writer);
        let title = Rectangle::new(
            width,
            16,
            1,
            1,
            PixelColor::White,
            PixelColor::White,
            PixelColor::Black,
        );
        title.draw(1, 1, &title.inside_pos(1, 1), &mut writer);
        let close_btn = Rectangle::new(
            16,
            16,
            1,
            0,
            PixelColor::Black,
            PixelColor::Black,
            PixelColor::White,
        );
        close_btn.draw(
            width - 17,
            2,
            &close_btn.inside_pos(width - 17, 2),
            &mut writer,
        );
        let info = Rectangle::new(
            width,
            height,
            1,
            1,
            PixelColor::White,
            PixelColor::Black,
            PixelColor::White,
        );
        info.draw(1, 19, &info.inside_pos(1, 19), &mut writer);
        let mut title_writer = PartialWriter::new(writer.clone(), title.inside_pos(1, 1));
        let mut close_btn_writer =
            PartialWriter::new(writer.clone(), close_btn.inside_pos(width - 17, 2));
        for x in 0..15 {
            for y in 0..15 {
                let tile_x = x / 8;
                let tile_y = y / 8;
                let tile_idx = tile_y * 2 + tile_x;
                let offset_x = x % 8;
                let offset_y = y % 8;
                if (CLOSE_BUTTON_IMG[tile_idx][offset_y] << offset_x) & 0x80 != 0 {
                    close_btn_writer.write(x, y, PixelColor::White);
                } else {
                    close_btn_writer.write(x, y, PixelColor::Black);
                }
            }
        }
        write_str(
            0,
            0,
            name,
            PixelColor::Black,
            PixelColor::White,
            &mut title_writer,
        );
        writer.set_button(close_btn.outside_pos(width - 17, 2));
        writer.set_title(title.outside_pos(1, 1));
        Self {
            writer,
            width,
            height,
            title,
            info,
        }
    }

    pub fn close(&self) {
        self.writer.close();
    }

    pub fn title(&self) -> PartialWriter<WindowWriter> {
        PartialWriter::new(self.writer.clone(), self.title.inside_pos(1, 1))
    }

    pub fn info(&self) -> PartialWriter<WindowWriter> {
        PartialWriter::new(self.writer.clone(), self.info.inside_pos(1, 19))
    }

    pub fn pop_event(&self) -> Option<Event> {
        self.writer.pop_event()
    }

    pub fn window_id(&self) -> usize {
        self.writer.0.lock().id
    }
}

impl Writable for WindowFrame {
    fn write(&mut self, x: usize, y: usize, color: PixelColor) {
        self.writer.write(x + 2, y + 22, color);
    }

    fn write_buf(&mut self, offset_x: usize, offset_y: usize, buffer: &[u32]) {
        self.writer.write_buf(offset_x + 2, offset_y + 22, buffer);
    }
}

impl Movable for WindowFrame {
    fn move_(&mut self, offset_x: isize, offset_y: isize) {
        self.writer.move_range(
            offset_x,
            offset_y,
            Area {
                x: 2,
                y: 22,
                width: self.width,
                height: self.height,
            },
        );
        // self.writer.move_(offset_x, offset_y)
    }

    fn move_range(&mut self, offset_x: isize, offset_y: isize, area: Area) {
        self.writer.move_range(offset_x, offset_y, area);
    }
}
