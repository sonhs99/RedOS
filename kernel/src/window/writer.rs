use alloc::vec;
use alloc::{sync::Arc, vec::Vec};
use log::debug;

use crate::task::running_task;
use crate::{
    graphic::{get_graphic, PixelColor},
    sync::Mutex,
    utility::abs,
};

use super::event::Event;
use super::{
    request_update_by_area, Area, Drawable, Movable, Window, WindowComponent, Writable,
    WINDOW_MANAGER,
};

pub(crate) struct FrameBuffer {
    width: usize,
    height: usize,
    buffer: Vec<u32>,
}

impl FrameBuffer {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            buffer: vec![0; width * height],
        }
    }
}

impl Writable for FrameBuffer {
    fn write(&mut self, x: usize, y: usize, color: PixelColor) {
        if x < self.width && y < self.height {
            self.buffer[y * self.width + x] = color.as_u32()
        }
    }

    fn write_buf(&mut self, offset_x: usize, offset_y: usize, buffer: &[u32]) {
        if offset_y < self.height {
            let len = if buffer.len() < self.width - offset_x {
                buffer.len()
            } else {
                self.width - offset_x
            };
            let offset = offset_y * self.width + offset_x;
            self.buffer[offset..offset + len].copy_from_slice(&buffer[..len]);
        }
    }

    fn write_id(&self) -> Option<usize> {
        None
    }
}

impl Drawable for FrameBuffer {
    fn draw(&self, offset_x: usize, offset_y: usize, area: &Area, writer: &mut impl Writable) {
        let end = (area.x + area.width).clamp(0, self.width);
        for (y, chunk) in self.buffer.chunks(self.width).enumerate() {
            if y < area.y + self.height && y >= area.y {
                writer.write_buf(offset_x + area.x, offset_y + y, &chunk[area.x..end]);
            }
        }
    }
}

#[derive(Clone)]
pub struct WindowWriter(pub(crate) Arc<Mutex<Window>>);

impl WindowWriter {
    pub fn close(self) {
        let area = {
            let mut manager = WINDOW_MANAGER.lock();
            let window = self.0.lock();
            let id = window.id;
            let area = manager.get_layer(id).unwrap().area();
            manager.remove(id);
            area
        };
        request_update_by_area(area);
    }

    pub fn set_title(&self, area: Area) {
        self.0.lock().title_area = Some(area);
    }

    pub fn set_button(&self, area: Area) {
        self.0.lock().button_area = Some(area);
    }

    pub fn get_area(&self, x: usize, y: usize) -> WindowComponent {
        if let Some(area) = self.0.lock().button_area {
            if area.is_in(x, y) {
                return WindowComponent::Close;
            }
        }
        if let Some(area) = self.0.lock().title_area {
            if area.is_in(x, y) {
                return WindowComponent::Title;
            }
        }
        WindowComponent::Body
    }

    pub fn push_event(&self, event: Event) {
        let _ = self.0.lock().event_queue.enqueue(event);
    }

    pub fn pop_event(&self) -> Option<Event> {
        self.0.lock().event_queue.dequeue().ok()
    }
}

impl Writable for WindowWriter {
    fn write(&mut self, x: usize, y: usize, color: PixelColor) {
        let mut window = self.0.lock();
        if x < window.width && y < window.height {
            let width = window.width;
            window.buffer[y * width + x] = color.as_u32()
        }
    }

    fn write_buf(&mut self, offset_x: usize, offset_y: usize, buffer: &[u32]) {
        let mut window = self.0.lock();
        if offset_y < window.height {
            let len = if buffer.len() < window.width - offset_x {
                buffer.len()
            } else {
                window.width - offset_x
            };
            let offset = offset_y * window.width + offset_x;
            window.buffer[offset..offset + len].copy_from_slice(&buffer[..len]);
        }
    }

    fn write_id(&self) -> Option<usize> {
        Some(self.0.lock().id)
    }
}

impl Movable for WindowWriter {
    fn move_(&mut self, offset_x: isize, offset_y: isize) {
        let width = self.0.lock().width;
        let height = self.0.lock().height;
        self.move_range(
            offset_x,
            offset_y,
            Area {
                x: 0,
                y: 0,
                width,
                height,
            },
        )
    }

    fn move_range(&mut self, offset_x: isize, offset_y: isize, area: Area) {
        let mut window = self.0.lock();

        let x1_a = abs(offset_x);
        let y1_a = abs(offset_y);
        let window_width = window.width;
        let width = if window.width > area.x + area.width {
            area.width
        } else {
            window.width - area.x
        };
        let height = if window.height > area.y + area.height {
            area.height
        } else {
            window.height - area.y
        };

        let width_len = (width - x1_a);
        let height_len = (height - y1_a);

        // assert!(area.y + height_len >= window.width);

        let buffer = &mut window.buffer;
        if offset_y > 0 {
            for idx_y in (0..height_len).rev() {
                let offset_dst = window_width * (idx_y + area.y);
                let offset_src = window_width * (idx_y + area.y - y1_a);
                if offset_x > 0 {
                    let offset_src = offset_src - x1_a;
                    for idx_x in 0..width_len {
                        if idx_x >= x1_a && idx_y >= y1_a {
                            buffer[offset_dst + idx_x + area.x] =
                                buffer[offset_src + idx_x + area.x];
                        } else {
                            buffer[offset_dst + idx_x + area.x] = 0;
                        }
                    }
                } else {
                    let offset_src = offset_src + x1_a;
                    for idx_x in (0..width_len).rev() {
                        if idx_x < width - x1_a && idx_y >= y1_a {
                            buffer[offset_dst + idx_x + area.x] =
                                buffer[offset_src + idx_x + area.x];
                        } else {
                            buffer[offset_dst + idx_x + area.x] = 0;
                        }
                    }
                }
            }
        } else {
            for idx_y in 0..height_len {
                let offset_dst = window_width * (idx_y + area.y);
                let offset_src = window_width * (idx_y + area.y + y1_a);
                if offset_x > 0 {
                    let offset_src = offset_src - x1_a;
                    for idx_x in 0..width_len {
                        if idx_x >= x1_a && idx_y < height - y1_a {
                            buffer[offset_dst + idx_x + area.x] =
                                buffer[offset_src + idx_x + area.x];
                        } else {
                            buffer[offset_dst + idx_x + area.x] = 0;
                        }
                    }
                } else {
                    let offset_src = offset_src + x1_a;
                    for idx_x in (0..width_len).rev() {
                        if idx_x < width - x1_a && idx_y < height - y1_a {
                            buffer[offset_dst + idx_x + area.x] =
                                buffer[offset_src + idx_x + area.x];
                        } else {
                            buffer[offset_dst + idx_x + area.x] = 0;
                        }
                    }
                }
            }
        }
    }
}

pub struct PartialWriter<T: Writable> {
    writer: T,
    area: Area,
}

impl<T: Writable> PartialWriter<T> {
    pub fn new(writer: T, area: Area) -> Self {
        Self { writer, area }
    }

    pub const fn area(&self) -> Area {
        self.area
    }
}

impl<T: Writable> Writable for PartialWriter<T> {
    fn write(&mut self, x: usize, y: usize, color: PixelColor) {
        if x < self.area.width && y < self.area.height {
            self.writer.write(x + self.area.x, y + self.area.y, color)
        }
    }

    fn write_buf(&mut self, offset_x: usize, offset_y: usize, buffer: &[u32]) {
        if offset_x < self.area.width && offset_y < self.area.height {
            let buf = if self.area.width - offset_x > buffer.len() {
                buffer
            } else {
                &buffer[..(self.area.width - offset_x)]
            };
            self.writer
                .write_buf(offset_x + self.area.x, offset_y + self.area.y, buf);
        }
    }

    fn write_id(&self) -> Option<usize> {
        self.writer.write_id()
    }
}

impl<T: Movable + Writable> Movable for PartialWriter<T> {
    fn move_(&mut self, offset_x: isize, offset_y: isize) {
        self.move_range(offset_x, offset_y, self.area);
    }

    fn move_range(&mut self, offset_x: isize, offset_y: isize, area: Area) {
        self.writer.move_range(offset_x, offset_y, self.area);
    }
}
