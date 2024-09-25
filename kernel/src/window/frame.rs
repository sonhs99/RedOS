use alloc::vec;

use crate::{font::write_ascii, graphic::PixelColor};

use super::{
    component::Button,
    create_window, create_window_pos,
    draw::{draw_line, draw_rect, draw_str, Point},
    event::Event,
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
}

impl WindowFrame {
    pub(crate) const WINDOW_PALETTE_FRAME: PixelColor = PixelColor(109, 218, 22);
    pub(crate) const WINDOW_PALETTE_TITLE: PixelColor = PixelColor::White;
    pub(crate) const WINDOW_PALETTE_TITLE_BACKGROUND: PixelColor = PixelColor(79, 204, 11);
    pub(crate) const WINDOW_PALETTE_BUTTON: PixelColor = PixelColor::White;
    pub(crate) const WINDOW_PALETTE_BUTTON_BORDER1: PixelColor = PixelColor(229, 229, 229);
    pub(crate) const WINDOW_PALETTE_BUTTON_BORDER2: PixelColor = PixelColor(86, 86, 86);
    pub(crate) const WINDOW_PALETTE_BODY: PixelColor = PixelColor::Black;
    pub(crate) const WINDOW_PALETTE_BODY_BACKGORUND: PixelColor = PixelColor::White;

    pub fn new_pos(x: usize, y: usize, width: usize, height: usize, name: &str) -> WindowFrame {
        let mut writer = create_window_pos(x, y, width + 4, height + 23);
        Self::inner_new(writer, width + 4, height + 23, name)
    }
    pub fn new(width: usize, height: usize, name: &str) -> WindowFrame {
        let mut writer = create_window(width + 4, height + 23);
        Self::inner_new(writer, width + 4, height + 23, name)
    }

    fn inner_new(mut writer: WindowWriter, width: usize, height: usize, name: &str) -> Self {
        // Border
        draw_rect(
            Point(0, 0),
            Point(width - 1, height - 1),
            Self::WINDOW_PALETTE_FRAME,
            false,
            &mut writer,
        );
        draw_rect(
            Point(1, 1),
            Point(width - 2, height - 2),
            Self::WINDOW_PALETTE_FRAME,
            false,
            &mut writer,
        );

        // Title
        draw_rect(
            Point(0, 3),
            Point(width - 2, 21),
            Self::WINDOW_PALETTE_TITLE_BACKGROUND,
            true,
            &mut writer,
        );

        draw_str(
            Point(6, 3),
            name,
            Self::WINDOW_PALETTE_TITLE,
            Self::WINDOW_PALETTE_TITLE_BACKGROUND,
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
            Self::WINDOW_PALETTE_BUTTON_BORDER1,
            Self::WINDOW_PALETTE_BUTTON_BORDER2,
            Self::WINDOW_PALETTE_BUTTON,
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
            Self::WINDOW_PALETTE_BODY_BACKGORUND,
            true,
            &mut writer,
        );

        writer.set_button(Area::new(width - 20, 1, 18, 18));
        writer.set_title(Area::new(0, 6, width - 2, 18));

        Self {
            writer,
            width,
            height,
            title: Area::new(0, 3, width - 1, 18),
            body: Area::new(2, 22, width - 4, height - 23),
        }
    }

    pub fn close(&self) {
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
}
