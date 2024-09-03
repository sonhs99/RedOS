pub mod frame;

use core::ops::DerefMut;

use alloc::vec::Vec;
use alloc::{sync::Arc, vec};
use hashbrown::HashMap;
use log::{debug, info};

use crate::sync::StaticCell;
use crate::utility::abs;
use crate::{
    graphic::{get_graphic, GraphicWriter, PixelColor},
    sync::{Mutex, MutexGuard, OnceLock},
    utility::random,
};

pub trait Drawable {
    fn draw(&self, offset_x: usize, offset_y: usize, writer: &mut impl Writable);
}

pub trait Writable {
    fn write(&mut self, x: usize, y: usize, color: PixelColor);
    fn write_buf(&mut self, offset_x: usize, offset_y: usize, buffer: &[u32]);
}

pub trait Movable {
    fn move_(&mut self, offset_x: isize, offset_y: isize);
    fn move_range(&mut self, offset_x: isize, offset_y: isize, area: Rectangle);
}

struct Rectangle {
    x: usize,
    y: usize,
    width: usize,
    height: usize,
}

impl Rectangle {
    pub const fn size(&self) -> usize {
        self.width * self.height
    }

    pub fn conjoint(&self, rect: &Rectangle) -> Option<Rectangle> {
        let x1 = if self.x > rect.x { self.x } else { rect.x };
        let y1 = if self.y > rect.y { self.y } else { rect.y };
        let x2 = if self.x + self.width < rect.x + rect.width {
            self.x + self.width
        } else {
            rect.x + rect.width
        };
        let y2 = if self.y + self.height < rect.y + rect.height {
            self.y + self.height
        } else {
            rect.y + rect.height
        };

        if (x2 - x1) * (y2 - y1) != 0 {
            Some(Self {
                x: x1,
                y: y1,
                width: x2 - x1,
                height: y2 - y1,
            })
        } else {
            None
        }
    }
}

// struct DrawBitmap {

// }

struct FrameBuffer {
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
}

impl Drawable for FrameBuffer {
    fn draw(&self, offset_x: usize, offset_y: usize, writer: &mut impl Writable) {
        for (y, chunk) in self.buffer.chunks(self.width).enumerate() {
            writer.write_buf(offset_x, offset_y + y, chunk);
        }
    }
}

pub struct Window {
    width: usize,
    height: usize,
    transparent_color: Option<u32>,
    buffer: Vec<u32>,
    update: bool,
}

impl Window {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            transparent_color: None,
            buffer: vec![0; width * height],
            update: true,
        }
    }

    pub fn set_transparent(&mut self, color: PixelColor) {
        self.transparent_color = Some(color.as_u32());
    }

    pub const fn width(&self) -> usize {
        self.width
    }

    pub const fn height(&self) -> usize {
        self.height
    }

    fn take_update(&mut self) -> bool {
        let flag = self.update;
        self.update = false;
        flag
    }

    pub fn need_update(&mut self) {
        self.update = true;
    }
}

impl Drawable for Window {
    fn draw(&self, offset_x: usize, offset_y: usize, writer: &mut impl Writable) {
        if let Some(transparent) = self.transparent_color {
            for (idx, &color) in self.buffer.iter().enumerate() {
                let x = idx % self.height;
                let y = idx / self.height;
                if color != transparent {
                    writer.write(x + offset_x, y + offset_y, color.into());
                }
            }
        } else {
            for (y, chunk) in self.buffer.chunks(self.width).enumerate() {
                writer.write_buf(offset_x, offset_y + y, chunk);
            }
        }
    }
}

pub struct WindowWriter(Arc<Mutex<Window>>);

impl Writable for WindowWriter {
    fn write(&mut self, x: usize, y: usize, color: PixelColor) {
        let mut window = self.0.lock();
        window.need_update();
        if x < window.width && y < window.height {
            let width = window.width;
            window.buffer[y * width + x] = color.as_u32()
        }
    }

    fn write_buf(&mut self, offset_x: usize, offset_y: usize, buffer: &[u32]) {
        let mut window = self.0.lock();
        window.need_update();
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
}

impl Movable for WindowWriter {
    fn move_(&mut self, offset_x: isize, offset_y: isize) {
        let width = self.0.lock().width;
        let height = self.0.lock().height;
        self.move_range(
            offset_x,
            offset_y,
            Rectangle {
                x: 0,
                y: 0,
                width,
                height,
            },
        )
    }

    fn move_range(&mut self, offset_x: isize, offset_y: isize, area: Rectangle) {
        let mut window = self.0.lock();
        window.need_update();

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

pub struct Layer {
    x: usize,
    y: usize,
    window: Arc<Mutex<Window>>,
    relocatable: bool,
}

impl Layer {
    pub fn new(x: usize, y: usize, width: usize, height: usize, relocatable: bool) -> Self {
        Self {
            x,
            y,
            window: Arc::new(Mutex::new(Window::new(width, height))),
            relocatable,
        }
    }

    pub fn move_(&mut self, x: usize, y: usize) {
        self.x += x;
        self.y += y;
    }

    pub fn area(&self) -> Rectangle {
        Rectangle {
            x: self.x,
            y: self.y,
            width: self.window.without_lock().width,
            height: self.window.without_lock().height,
        }
    }

    pub fn take_update(&mut self) -> bool {
        self.window.lock().take_update()
    }

    pub fn writer(&mut self) -> WindowWriter {
        WindowWriter(self.window.clone())
    }
}

impl Drawable for Layer {
    #[inline(never)]
    fn draw(&self, offset_x: usize, offset_y: usize, writer: &mut impl Writable) {
        self.window
            .lock()
            .draw(offset_x + self.x, offset_y + self.y, writer);
    }
}

pub struct WindowManager {
    layers: HashMap<usize, Layer>,
    stack: Vec<usize>,
    global: FrameBuffer,
    resolution: (usize, usize),
    focus: usize,
    count: usize,
}

impl WindowManager {
    pub fn new(resolution: (usize, usize)) -> Self {
        Self {
            layers: HashMap::new(),
            stack: Vec::new(),
            global: FrameBuffer::new(resolution.0, resolution.1),
            resolution,
            focus: 0,
            count: 0,
        }
    }

    pub fn new_layer(&mut self, width: usize, height: usize, relocatable: bool) -> &mut Layer {
        let (x, y) = (
            random() % self.resolution.0 as u64,
            random() % self.resolution.1 as u64,
        );
        let id = self.count;
        self.count += 1;
        self.layers.insert(
            id,
            Layer::new(x as usize, y as usize, width, height, relocatable),
        );
        self.stack.push(id);
        self.focus = self.count;
        self.layers.get_mut(&id).expect("Not Found")
    }

    pub fn new_layer_pos(
        &mut self,
        x: usize,
        y: usize,
        width: usize,
        height: usize,
        relocatable: bool,
    ) -> &mut Layer {
        let id = self.count;
        self.count += 1;
        self.layers.insert(
            id,
            Layer::new(x as usize, y as usize, width, height, relocatable),
        );
        self.stack.push(id);
        self.focus = self.count;
        self.layers.get_mut(&id).expect("Not Found")
    }

    pub fn focus(&mut self, id: usize) {
        if !self.layers.contains_key(&id) {
            return;
        }
        if let Some((idx, _)) = self.stack.iter().enumerate().find(|(idx, &x)| x == id) {
            self.stack.remove(idx);
            self.stack.push(id);
            self.focus = id;
        }
    }

    pub fn visibility(&mut self, id: usize, visible: bool) {
        if !self.layers.contains_key(&id) {
            return;
        }

        if visible {
            if self
                .stack
                .iter()
                .enumerate()
                .find(|(idx, &x)| x == id)
                .is_none()
            {
                self.stack.push(id);
                self.focus = id;
            }
        } else {
            if let Some((idx, _)) = self.stack.iter().enumerate().find(|(idx, &x)| x == id) {
                self.stack.remove(idx);
            }
        }
    }

    pub fn remove(&mut self, id: usize) {
        if let Some((idx, _)) = self.stack.iter().enumerate().find(|(idx, &x)| x == id) {
            self.stack.remove(idx);
            self.focus = id;
        }

        self.layers.remove(&id);
    }

    pub fn get_layer(&mut self, id: usize) -> &mut Layer {
        self.layers.get_mut(&id).unwrap()
    }

    pub fn render(&mut self, writer: &mut impl Writable) {
        let mut flag = false;
        for layer_id in self.stack.iter() {
            let layer = self.layers.get_mut(layer_id).expect("Not Found");
            flag = flag || layer.take_update();
            if flag {
                layer.draw(0, 0, &mut self.global);
            }
        }

        self.global.draw(0, 0, writer);
    }
}

impl Writable for GraphicWriter {
    fn write(&mut self, x: usize, y: usize, color: PixelColor) {
        GraphicWriter::write(&self, x, y, color);
    }

    fn write_buf(&mut self, offset_x: usize, offset_y: usize, buffer: &[u32]) {
        GraphicWriter::write_buf(&self, offset_x, offset_y, buffer);
    }
}

impl<T: Writable> Writable for MutexGuard<'_, T> {
    fn write(&mut self, x: usize, y: usize, color: PixelColor) {
        self.deref_mut().write(x, y, color);
    }

    fn write_buf(&mut self, offset_x: usize, offset_y: usize, buffer: &[u32]) {
        self.deref_mut().write_buf(offset_x, offset_y, buffer);
    }
}

static WINDOW_MANAGER: OnceLock<Mutex<WindowManager>> = OnceLock::new();

pub fn init_window(resolution: (usize, usize)) {
    WINDOW_MANAGER.get_or_init(|| Mutex::new(WindowManager::new(resolution)));

    let mut manager = WINDOW_MANAGER.lock();
    let bg_layer = manager.new_layer_pos(0, 0, resolution.0, resolution.1, false);
    let mut writer = bg_layer.writer();
    let buffer = vec![PixelColor::Blue.as_u32(); resolution.0];
    for y in 0..resolution.1 {
        writer.write_buf(0, y, &buffer);
    }

    let nav_bar = manager.new_layer_pos(0, 0, resolution.0, 18, false);
    let mut writer = nav_bar.writer();
    let buffer = vec![PixelColor::Black.as_u32(); resolution.0];
    for y in 0..18 {
        writer.write_buf(0, y, &buffer);
    }
    manager.render(&mut get_graphic().lock());
}

pub fn new_layer(width: usize, height: usize) -> WindowWriter {
    WINDOW_MANAGER
        .lock()
        .new_layer(width, height, true)
        .writer()
}

pub fn new_layer_pos(x: usize, y: usize, width: usize, height: usize) -> WindowWriter {
    WINDOW_MANAGER
        .lock()
        .new_layer_pos(x, y, width, height, true)
        .writer()
}

pub fn render() {
    WINDOW_MANAGER.lock().render(&mut get_graphic().lock());
}
