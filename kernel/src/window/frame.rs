use alloc::vec;

use crate::{font::write_ascii, graphic::PixelColor};

use super::{
    component::{Button, Palette},
    create_window, create_window_pos,
    draw::{draw_line, draw_rect, draw_str, Point},
    event::Event,
    request_update_by_id,
    writer::PartialWriter,
    Area, Drawable, Movable, WindowWriter, Writable,
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
    title: Area,
    body: Area,
    palette: Palette,
}

impl WindowFrame {
    pub(crate) const WINDOW_PALETTE_FRAME: usize = 0;
    pub(crate) const WINDOW_PALETTE_TITLE: usize = 1;
    pub(crate) const WINDOW_PALETTE_TITLE_BACKGROUND: usize = 2;
    pub(crate) const WINDOW_PALETTE_BUTTON: usize = 3;
    pub(crate) const WINDOW_PALETTE_BUTTON_BORDER1: usize = 4;
    pub(crate) const WINDOW_PALETTE_BUTTON_BORDER2: usize = 5;
    pub(crate) const WINDOW_PALETTE_BODY: usize = 6;
    pub(crate) const WINDOW_PALETTE_BODY_BACKGORUND: usize = 7;

    pub fn new_pos(x: usize, y: usize, width: usize, height: usize, name: &str) -> WindowFrame {
        let mut writer = create_window_pos(x, y, width + 4, height + 23);
        Self::inner_new(writer, width + 4, height + 23, name)
    }
    pub fn new(width: usize, height: usize, name: &str) -> WindowFrame {
        let mut writer = create_window(width + 4, height + 23);
        Self::inner_new(writer, width + 4, height + 23, name)
    }

    fn inner_new(mut writer: WindowWriter, width: usize, height: usize, name: &str) -> Self {
        let palette = Palette::new(vec![
            PixelColor(109, 218, 22),
            PixelColor::White,
            PixelColor(79, 204, 11),
            PixelColor::White,
            PixelColor(229, 229, 229),
            PixelColor(86, 86, 86),
            PixelColor::Black,
            PixelColor::White,
        ]);
        // Border
        draw_rect(
            Point(0, 0),
            Point(width - 1, height - 1),
            palette.get(Self::WINDOW_PALETTE_FRAME).unwrap(),
            false,
            &mut writer,
        );
        draw_rect(
            Point(1, 1),
            Point(width - 2, height - 2),
            palette.get(Self::WINDOW_PALETTE_FRAME).unwrap(),
            false,
            &mut writer,
        );

        // Title
        draw_rect(
            Point(0, 3),
            Point(width - 2, 21),
            palette.get(Self::WINDOW_PALETTE_TITLE_BACKGROUND).unwrap(),
            true,
            &mut writer,
        );

        draw_str(
            Point(6, 3),
            name,
            palette.get(Self::WINDOW_PALETTE_TITLE).unwrap(),
            palette.get(Self::WINDOW_PALETTE_TITLE_BACKGROUND).unwrap(),
            &mut writer,
        );

        // Title - Volume
        draw_line(
            Point(1, 1),
            Point(width - 1, 1),
            PixelColor(183, 249, 171),
            &mut writer,
        );
        draw_line(
            Point(1, 2),
            Point(width - 1, 2),
            PixelColor(150, 210, 140),
            &mut writer,
        );
        draw_line(
            Point(1, 2),
            Point(1, 20),
            PixelColor(183, 249, 171),
            &mut writer,
        );
        draw_line(
            Point(2, 2),
            Point(2, 20),
            PixelColor(150, 210, 140),
            &mut writer,
        );

        let close_btn = Button::new(
            14,
            14,
            2,
            0,
            palette.get(Self::WINDOW_PALETTE_BUTTON_BORDER1).unwrap(),
            palette.get(Self::WINDOW_PALETTE_BUTTON_BORDER2).unwrap(),
            palette.get(Self::WINDOW_PALETTE_BUTTON).unwrap(),
            PixelColor::Black,
        );
        close_btn.draw(width - 20, 1, &Area::new(0, 0, width, height), &mut writer);

        // Close Button - Cross
        draw_line(
            Point(width - 20 + 4, 1 + 4),
            Point(width - 2 - 4, 19 - 4),
            PixelColor(71, 199, 21),
            &mut writer,
        );
        draw_line(
            Point(width - 20 + 5, 1 + 4),
            Point(width - 2 - 4, 19 - 5),
            PixelColor(71, 199, 21),
            &mut writer,
        );
        draw_line(
            Point(width - 20 + 4, 1 + 5),
            Point(width - 2 - 5, 19 - 4),
            PixelColor(71, 199, 21),
            &mut writer,
        );
        draw_line(
            Point(width - 20 + 4, 19 - 4),
            Point(width - 2 - 4, 1 + 4),
            PixelColor(71, 199, 21),
            &mut writer,
        );
        draw_line(
            Point(width - 20 + 5, 19 - 4),
            Point(width - 2 - 4, 1 + 5),
            PixelColor(71, 199, 21),
            &mut writer,
        );
        draw_line(
            Point(width - 20 + 4, 19 - 5),
            Point(width - 2 - 5, 1 + 4),
            PixelColor(71, 199, 21),
            &mut writer,
        );

        draw_rect(
            Point(2, 22),
            Point(width - 2, height - 2),
            palette.get(Self::WINDOW_PALETTE_BODY_BACKGORUND).unwrap(),
            true,
            &mut writer,
        );

        writer.set_button(Area::new(width - 20, 1, 18, 18));
        writer.set_title(Area::new(0, 6, width - 2, 18));

        request_update_by_id(writer.0.lock().id);

        Self {
            writer,
            width,
            height,
            title: Area::new(0, 0, width, 21),
            body: Area::new(2, 22, width - 4, height - 24),
            palette,
        }
    }

    pub fn close(self) {
        self.writer.close();
    }

    pub fn title(&self) -> PartialWriter<WindowWriter> {
        PartialWriter::new(self.writer.clone(), self.title)
    }

    pub fn body(&self) -> PartialWriter<WindowWriter> {
        PartialWriter::new(self.writer.clone(), self.body)
    }

    pub fn pop_event(&self) -> Option<Event> {
        self.writer.pop_event()
    }

    pub fn window_id(&self) -> usize {
        self.writer.0.lock().id
    }

    pub fn set_background(&mut self, color: PixelColor) {
        self.palette
            .set(Self::WINDOW_PALETTE_BODY_BACKGORUND, color);
        draw_rect(
            Point(0, 0),
            Point(self.width, self.height),
            color,
            true,
            &mut self.body(),
        );
    }
}
