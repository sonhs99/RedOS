use alloc::vec;

use crate::graphic::PixelColor;

use super::{
    draw::{draw_line, draw_rect, Point},
    frame::WindowFrame,
    request_update_by_id, Area, Drawable, Writable,
};

pub struct Button {
    width: usize,
    height: usize,
    border: usize,
    margin: usize,
    border_color1: PixelColor,
    border_color2: PixelColor,
    background_color: PixelColor,
    foreground_color: PixelColor,
}

impl Button {
    pub fn new(
        width: usize,
        height: usize,
        border: usize,
        margin: usize,
        border_color1: PixelColor,
        border_color2: PixelColor,
        background_color: PixelColor,
        foreground_color: PixelColor,
    ) -> Self {
        Self {
            width,
            height,
            border,
            margin,
            border_color1,
            border_color2,
            background_color,
            foreground_color,
        }
    }

    pub fn new_default(width: usize, height: usize, border: usize, margin: usize) -> Self {
        Self::new(
            width,
            height,
            border,
            margin,
            WindowFrame::WINDOW_PALETTE_BUTTON_BORDER1,
            WindowFrame::WINDOW_PALETTE_BUTTON_BORDER2,
            WindowFrame::WINDOW_PALETTE_BUTTON,
            PixelColor::Black,
        )
    }

    pub fn area(&self, offset: Point) -> Area {
        Area::new(
            self.border + self.margin + offset.0,
            self.border + self.margin + offset.1,
            self.width,
            self.height,
        )
    }

    pub fn update_bg(&mut self, color: PixelColor) {
        self.background_color = color;
    }

    pub fn update_fg(&mut self, color: PixelColor) {
        self.foreground_color = color;
    }
}

impl Drawable for Button {
    fn draw(&self, offset_x: usize, offset_y: usize, area: &Area, writer: &mut impl Writable) {
        let width = 2 * (self.border + self.margin) + self.width;
        let height = 2 * (self.border + self.margin) + self.height;
        // Close Button
        draw_rect(
            Point(offset_x, offset_y),
            Point(offset_x + width, offset_y + height),
            self.background_color,
            true,
            writer,
        );

        // Close Button - Volume
        for idx in 0..self.border {
            draw_line(
                Point(offset_x + idx, offset_y + idx),
                Point(offset_x + width - idx, offset_y + idx),
                self.border_color1,
                writer,
            );
            draw_line(
                Point(offset_x + idx, offset_y + idx),
                Point(offset_x + idx, offset_y + height - idx),
                self.border_color1,
                writer,
            );
            draw_line(
                Point(offset_x + width - idx, offset_y + idx),
                Point(offset_x + width - idx, offset_y + height - idx),
                self.border_color2,
                writer,
            );
            draw_line(
                Point(offset_x + idx, offset_y + height - idx),
                Point(offset_x + width - idx, offset_y + height - idx),
                self.border_color2,
                writer,
            );
        }

        if let Some(id) = writer.write_id() {
            request_update_by_id(id);
        }
    }
}
