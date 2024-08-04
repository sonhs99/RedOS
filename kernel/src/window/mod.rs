use alloc::vec::Vec;

use crate::graphic::{GraphicWriter, PixelColor};

pub trait Drawable {
    fn draw(&self, offset_x: usize, offset_y: usize, writer: &mut impl Writable);
}

pub trait Writable {
    fn write(&mut self, x: usize, y: usize, color: PixelColor);
}

pub struct Window {
    width: usize,
    height: usize,
    transparent_color: Option<PixelColor>,
    buffer: Vec<PixelColor>,
}

impl Window {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            transparent_color: None,
            buffer: Vec::with_capacity(width * height),
        }
    }

    pub fn set_transparent(&mut self, color: PixelColor) {
        self.transparent_color = Some(color);
    }

    pub const fn width(&self) -> usize {
        self.width
    }

    pub const fn height(&self) -> usize {
        self.height
    }

    pub fn writer(&mut self) -> WindowWriter {
        WindowWriter(self)
    }
}

impl Drawable for Window {
    fn draw(&self, offset_x: usize, offset_y: usize, writer: &mut impl Writable) {
        for (idx, &color) in self.buffer.iter().enumerate() {
            let x = idx % self.width;
            let y = idx / self.width;
            if let Some(transparent) = self.transparent_color {
                if color != transparent {
                    writer.write(x + offset_x, y + offset_y, color);
                }
            } else {
                writer.write(x + offset_x, y + offset_y, color);
            }
        }
    }
}

pub struct WindowWriter<'a>(&'a mut Window);

impl<'a> Writable for WindowWriter<'a> {
    fn write(&mut self, x: usize, y: usize, color: PixelColor) {
        self.0.buffer[y * self.0.width + x] = color
    }
}

pub struct Layer {
    x: usize,
    y: usize,
    window: Window,
}

impl Layer {
    pub fn new(x: usize, y: usize, width: usize, height: usize) -> Self {
        Self {
            x,
            y,
            window: Window::new(width, height),
        }
    }

    pub fn move_(&mut self, x: usize, y: usize) {
        self.x += x;
        self.y += y;
    }
}

impl Drawable for Layer {
    fn draw(&self, offset_x: usize, offset_y: usize, writer: &mut impl Writable) {
        self.window
            .draw(offset_x + self.x, offset_y + self.y, writer)
    }
}
