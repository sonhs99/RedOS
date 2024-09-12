pub mod component;
pub mod event;
pub mod frame;

use core::ops::DerefMut;

use alloc::vec::Vec;
use alloc::{sync::Arc, vec};
use component::write_str;
use event::{DestId, Event, EventType, MouseEvent, EVENT_QUEUE_SIZE};
use frame::WindowFrame;
use hashbrown::HashMap;
use log::{debug, info};

use crate::device::driver::keyboard::get_keystate_unblocked;
use crate::device::driver::mouse::{get_mouse_state, get_mouse_state_unblocked};
use crate::font::write_ascii;
use crate::queue::VecQueue;
use crate::sync::StaticCell;
use crate::utility::abs;
use crate::{
    graphic::{get_graphic, GraphicWriter, PixelColor},
    sync::{Mutex, MutexGuard, OnceLock},
    utility::random,
};

const MOUSE_CURSER: [[u8; 16]; 6] = [
    [
        0b1000_0000,
        0b1000_0000,
        0b1100_0000,
        0b1100_0000,
        0b1110_0000,
        0b1010_0000,
        0b1111_0000,
        0b1001_0000,
        0b1111_1000,
        0b1000_1000,
        0b1111_1100,
        0b1000_0100,
        0b1111_1110,
        0b1000_0010,
        0b1111_1111,
        0b1000_0001,
    ],
    [
        0b0000_0000,
        0b0000_0000,
        0b0000_0000,
        0b0000_0000,
        0b0000_0000,
        0b0000_0000,
        0b0000_0000,
        0b0000_0000,
        0b0000_0000,
        0b0000_0000,
        0b0000_0000,
        0b0000_0000,
        0b0000_0000,
        0b0000_0000,
        0b0000_0000,
        0b0000_0000,
    ],
    [
        0b1111_1111,
        0b1000_0000,
        0b1111_1111,
        0b1000_0000,
        0b1111_1111,
        0b1000_0000,
        0b1111_1111,
        0b1000_0000,
        0b1111_1111,
        0b1000_0000,
        0b1111_1111,
        0b1000_0000,
        0b1111_1111,
        0b1000_0000,
        0b1111_1111,
        0b1000_0000,
    ],
    [
        0b1000_0000,
        0b1000_0000,
        0b1100_0000,
        0b0100_0000,
        0b1110_0000,
        0b0010_0000,
        0b1111_0000,
        0b0001_0000,
        0b1111_1000,
        0b0000_1000,
        0b1111_1100,
        0b0000_0100,
        0b1111_1110,
        0b1111_1110,
        0b0000_0000,
        0b0000_0000,
    ],
    [
        0b1111_1111,
        0b1000_0011,
        0b1111_1101,
        0b1000_0101,
        0b1111_1000,
        0b1000_1000,
        0b1111_0000,
        0b1001_0000,
        0b1110_0000,
        0b1010_0000,
        0b1100_0000,
        0b1100_0000,
        0b1000_0000,
        0b1000_0000,
        0b0000_0000,
        0b0000_0000,
    ],
    [
        0b1100_0000,
        0b0100_0000,
        0b1100_0000,
        0b0100_0000,
        0b1110_0000,
        0b1010_0000,
        0b1110_0000,
        0b1010_0000,
        0b0111_0000,
        0b0101_0000,
        0b0111_0000,
        0b0101_0010,
        0b0011_1000,
        0b0010_1000,
        0b0011_1000,
        0b0011_1000,
    ],
];

enum WindowComponent {
    Body,
    Title,
    Close,
}

pub trait Drawable {
    fn draw(&self, offset_x: usize, offset_y: usize, area: &Area, writer: &mut impl Writable);
}

pub trait Writable {
    fn write(&mut self, x: usize, y: usize, color: PixelColor);
    fn write_buf(&mut self, offset_x: usize, offset_y: usize, buffer: &[u32]);
}

pub trait Movable {
    fn move_(&mut self, offset_x: isize, offset_y: isize);
    fn move_range(&mut self, offset_x: isize, offset_y: isize, area: Area);
}

#[derive(Clone, Copy)]
pub struct Area {
    x: usize,
    y: usize,
    width: usize,
    height: usize,
}

impl Area {
    pub const fn new(x: usize, y: usize, width: usize, height: usize) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }
    pub const fn size(&self) -> usize {
        self.width * self.height
    }

    pub fn conjoint(&self, area: &Self) -> Option<Self> {
        if self.x + self.width < area.x
            || self.x > area.x + area.width
            || self.y + self.height < area.y
            || self.y > area.y + area.height
        {
            return None;
        }
        let x1 = if self.x > area.x { self.x } else { area.x };
        let y1 = if self.y > area.y { self.y } else { area.y };
        let x2 = if self.x + self.width < area.x + area.width {
            self.x + self.width
        } else {
            area.x + area.width
        };
        let y2 = if self.y + self.height < area.y + area.height {
            self.y + self.height
        } else {
            area.y + area.height
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

    pub fn union(&self, area: &Self) -> Self {
        let x1 = if self.x < area.x { self.x } else { area.x };
        let y1 = if self.y < area.y { self.y } else { area.y };
        let x2 = if self.x + self.width > area.x + area.width {
            self.x + self.width
        } else {
            area.x + area.width
        };
        let y2 = if self.y + self.height > area.y + area.height {
            self.y + self.height
        } else {
            area.y + area.height
        };
        Self::new(x1, y1, x2 - x1, y2 - y1)
    }

    pub fn offset_of(&self, base: &Self) -> Self {
        Self::new(self.x - base.x, self.y - base.y, self.width, self.height)
    }

    pub fn is_in(&self, x: usize, y: usize) -> bool {
        x < self.x + self.width && x >= self.x && y < self.y + self.height && y >= self.y
    }

    pub fn local(&self, x: usize, y: usize) -> (usize, usize) {
        (x - self.x, y - self.y)
    }

    pub fn global(&self, x: usize, y: usize) -> (usize, usize) {
        (x + self.x, y + self.y)
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
    fn draw(&self, offset_x: usize, offset_y: usize, area: &Area, writer: &mut impl Writable) {
        for (y, chunk) in self.buffer.chunks(self.width).enumerate() {
            if y < area.y + self.height && y >= area.y {
                writer.write_buf(offset_x, offset_y + y, &chunk[area.x..area.x + area.width]);
            }
        }
    }
}

pub struct Window {
    id: usize,
    width: usize,
    height: usize,
    transparent_color: Option<u32>,
    buffer: Vec<u32>,
    event_queue: VecQueue<Event>,
    update: bool,
    title_area: Option<Area>,
    button_area: Option<Area>,
}

impl Window {
    pub fn new(id: usize, width: usize, height: usize) -> Self {
        Self {
            id,
            width,
            height,
            transparent_color: None,
            buffer: vec![0; width * height],
            event_queue: VecQueue::new(Event::default(), EVENT_QUEUE_SIZE),
            update: true,
            title_area: None,
            button_area: None,
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
    fn draw(&self, offset_x: usize, offset_y: usize, area: &Area, writer: &mut impl Writable) {
        if let Some(transparent) = self.transparent_color {
            for (idx, &color) in self.buffer.iter().enumerate() {
                let x = idx % self.height;
                let y = idx / self.height;
                if area.is_in(x, y) && color != transparent {
                    writer.write(x + offset_x, y + offset_y, color.into());
                }
            }
        } else {
            for (y, chunk) in self.buffer[area.y * self.width..(area.y + area.height) * self.width]
                .chunks(self.width)
                .enumerate()
            {
                writer.write_buf(
                    offset_x + area.x,
                    offset_y + area.y + y,
                    &chunk[area.x..area.x + area.width],
                );
            }
        }
    }
}

#[derive(Clone)]
pub struct WindowWriter(Arc<Mutex<Window>>);

impl WindowWriter {
    pub fn close(&self) {
        let mut manager = WINDOW_MANAGER.lock();
        let id = self.0.lock().id;
        let area = manager.get_layer(id).area();
        manager.remove(id);
        manager.render(&area, &mut get_graphic().lock());
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

pub struct PartialWriter<T: Writable> {
    writer: T,
    area: Area,
}

impl<T: Writable> PartialWriter<T> {
    pub fn new(writer: T, area: Area) -> Self {
        Self { writer, area }
    }
}

impl<T: Writable> Writable for PartialWriter<T> {
    fn write(&mut self, x: usize, y: usize, color: PixelColor) {
        if x < self.area.x + self.area.width && y < self.area.y + self.area.height {
            self.writer.write(x + self.area.x, y + self.area.y, color)
        }
    }

    fn write_buf(&mut self, offset_x: usize, offset_y: usize, buffer: &[u32]) {
        if offset_x < self.area.x + self.area.width && offset_y < self.area.y + self.area.height {
            let buf = if self.area.width - offset_x > buffer.len() {
                buffer
            } else {
                &buffer[..(self.area.width - offset_x)]
            };
            self.writer
                .write_buf(offset_x + self.area.x, offset_y + self.area.y, buf);
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
    pub fn new(
        id: usize,
        x: usize,
        y: usize,
        width: usize,
        height: usize,
        relocatable: bool,
    ) -> Self {
        Self {
            x,
            y,
            window: Arc::new(Mutex::new(Window::new(id, width, height))),
            relocatable,
        }
    }

    pub fn move_(&mut self, dx: isize, dy: isize) {
        let id = self.window.lock().id;
        self.x = (self.x as isize + dx).abs() as usize;
        self.y = (self.y as isize + dy).abs() as usize;
        // debug!(
        //     "id={} Move to x={} y={}, dx={}, dy={}",
        //     id, self.x, self.y, dx, dy
        // );
        self.window.lock().need_update();
    }

    pub fn area(&self) -> Area {
        let window = self.window.lock();
        Area {
            x: self.x,
            y: self.y,
            width: window.width,
            height: window.height,
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
    fn draw(&self, offset_x: usize, offset_y: usize, area: &Area, writer: &mut impl Writable) {
        let my_area = self.area();
        if let Some(conjointed_area) = area.conjoint(&my_area) {
            let offset_area = conjointed_area.offset_of(&my_area);
            self.window
                .lock()
                .draw(offset_x + self.x, offset_y + self.y, &offset_area, writer);
        }
    }
}

pub struct WindowManager {
    layers: HashMap<usize, Layer>,
    stack: Vec<usize>,
    global: FrameBuffer,
    resolution: (usize, usize),
    event_queue: VecQueue<Event>,
    count: usize,
    mouse_x: usize,
    mouse_y: usize,
    update: bool,
    moving: bool,
}

impl WindowManager {
    pub fn new(resolution: (usize, usize)) -> Self {
        Self {
            layers: HashMap::new(),
            stack: Vec::new(),
            global: FrameBuffer::new(resolution.0, resolution.1),
            resolution,
            event_queue: VecQueue::new(Event::default(), EVENT_QUEUE_SIZE),
            count: 0,
            mouse_x: resolution.0 / 2,
            mouse_y: resolution.1 / 2,
            update: false,
            moving: false,
        }
    }

    pub fn new_layer(&mut self, width: usize, height: usize, relocatable: bool) -> &mut Layer {
        let (x, y) = (
            random() as usize % self.resolution.0,
            random() as usize % self.resolution.1,
        );
        self.new_layer_pos(x, y, width, height, relocatable)
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
        self.count = self.count.wrapping_add(1);
        self.layers
            .insert(id, Layer::new(id, x, y, width, height, relocatable));
        self.stack.push(id);
        self.get_layer(id).window.lock().need_update();
        self.layers.get_mut(&id).expect("Not Found")
    }

    pub fn focus(&mut self, id: usize) {
        if !self.layers.contains_key(&id) {
            return;
        }
        if !self.get_layer(id).relocatable {
            return;
        }
        if let Some((idx, _)) = self.stack.iter().enumerate().find(|(idx, &x)| x == id) {
            self.stack.remove(idx);
            self.stack.push(id);
        }
        self.get_layer(id).window.lock().need_update();
        // debug!("Top Layer id={id}");
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
            }
        } else {
            if let Some((idx, _)) = self.stack.iter().enumerate().find(|(idx, &x)| x == id) {
                self.stack.remove(idx);
            }
        }
        self.get_layer(id).window.lock().need_update();
    }

    pub const fn area(&self) -> Area {
        Area {
            x: 0,
            y: 0,
            width: self.resolution.0,
            height: self.resolution.1,
        }
    }

    pub fn remove(&mut self, id: usize) {
        for (idx, &window_id) in self.stack.iter().enumerate() {
            if window_id == id {
                self.stack.remove(idx);
                break;
            }
        }

        self.layers.remove(&id);
        self.update = true;
        // debug!("[WINDOW] Window ID={id} Removed");
    }

    pub fn get_layer(&mut self, id: usize) -> &mut Layer {
        self.layers.get_mut(&id).unwrap()
    }

    pub fn top_layer(&mut self) -> &mut Layer {
        let top_id = self.stack.last().unwrap();
        self.layers.get_mut(top_id).unwrap()
    }

    pub fn get_layer_id_from_point(&mut self, x: usize, y: usize) -> usize {
        for layer_id in self.stack.iter().rev() {
            let layer = self.layers.get_mut(layer_id).expect("Not Found");
            if layer.area().is_in(x, y) {
                return *layer_id;
            }
        }
        0
    }

    pub fn get_mouse(&self) -> (usize, usize) {
        (self.mouse_x, self.mouse_y)
    }

    fn render_inner(&mut self, area: &Area, writer: &mut impl Writable) {
        let mut flag = self.update;
        let mut area = area.clone();
        for layer_id in self.stack.iter() {
            let layer = self.layers.get_mut(layer_id).expect("Not Found");
            let update_flag = layer.take_update();
            if update_flag {
                area = layer.area().union(&area);
            }
            flag = flag || update_flag;
            if flag {
                layer.draw(0, 0, &area, &mut self.global);
            }
        }
        self.update = false;
        print_curser(self.mouse_x, self.mouse_y, &mut self.global);
    }

    pub fn render_mouse(
        &mut self,
        x: usize,
        y: usize,
        prev_area: Area,
        writer: &mut impl Writable,
    ) {
        self.mouse_x = x;
        self.mouse_y = y;
        self.update = true;
        self.render_inner(&prev_area, writer);
        self.global.draw(0, 0, &self.area(), writer);
    }

    pub fn render(&mut self, area: &Area, writer: &mut impl Writable) {
        self.render_inner(&area, writer);
        self.global.draw(0, 0, area, writer);
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
    let area = manager.area();
    manager.render(&area, &mut get_graphic().lock());
}

pub fn create_window(width: usize, height: usize) -> WindowWriter {
    WINDOW_MANAGER
        .lock()
        .new_layer(width, height, true)
        .writer()
}

pub fn create_window_pos(x: usize, y: usize, width: usize, height: usize) -> WindowWriter {
    WINDOW_MANAGER
        .lock()
        .new_layer_pos(x, y, width, height, true)
        .writer()
}

pub fn render() {
    let mut manager = WINDOW_MANAGER.lock();
    let area = manager.area();
    WINDOW_MANAGER
        .lock()
        .render(&area, &mut get_graphic().lock());
}

fn print_curser(x: usize, y: usize, writer: &mut impl Writable) {
    for dy in 0..24 {
        for dx in 0..16 {
            let tile_high = dy / 8;
            let tile_low = dx / 8;
            let tile_idx = tile_high << 1 | tile_low;
            let offset_x = dx % 8;
            let offset_y = dy % 8;
            let high = MOUSE_CURSER[tile_idx][offset_y * 2] >> (7 - offset_x) & 0x01;
            let low = MOUSE_CURSER[tile_idx][offset_y * 2 + 1] >> (7 - offset_x) & 0x01;
            match high << 1 | low {
                0b10 => writer.write(x + dx, y + dy, PixelColor::White),
                0b11 => writer.write(x + dx, y + dy, PixelColor::Black),
                _ => {}
            }
        }
    }
}

fn process_mouse() {
    if let Some(status) = get_mouse_state_unblocked() {
        // debug!("asdf");
        let mut manager = WINDOW_MANAGER.lock();

        let resolution = manager.resolution;
        let prev = manager.get_mouse();
        let mut area = Area::new(prev.0, prev.1, 16, 24);
        let mouse_x =
            (prev.0 as isize + status.x_v() as isize).clamp(0, resolution.0 as isize - 1) as usize;
        let mouse_y =
            (prev.1 as isize + status.y_v() as isize).clamp(0, resolution.1 as isize - 1) as usize;

        let window_id = manager.get_layer_id_from_point(mouse_x, mouse_y);
        let writer = manager.get_layer(window_id).writer();

        let mut is_button_changed = false;
        let (local_x, local_y) = manager.get_layer(window_id).area().local(mouse_x, mouse_y);

        let dx = mouse_x as isize - prev.0 as isize;
        let dy = mouse_y as isize - prev.1 as isize;

        for button in 0..8 {
            if status.pressed(button) {
                let changed = if button == 0 {
                    manager.focus(window_id);
                    match writer.get_area(local_x, local_y) {
                        WindowComponent::Body => false,
                        WindowComponent::Title => {
                            is_button_changed = true;
                            writer.push_event(Event::new(
                                DestId::One(window_id),
                                EventType::Window(event::WindowEvent::Move),
                            ));
                            // debug!("id={window_id} Drag Start");
                            area = manager.top_layer().area().union(&area);
                            manager.top_layer().move_(dx, dy);
                            manager.moving = true;
                            true
                        }
                        WindowComponent::Close => {
                            is_button_changed = true;
                            writer.push_event(Event::new(
                                DestId::One(window_id),
                                EventType::Window(event::WindowEvent::Close),
                            ));
                            true
                        }
                    }
                } else {
                    false
                };
                if !changed {
                    debug!("id={window_id} Pressed");
                    is_button_changed = true;

                    writer.push_event(Event::new(
                        DestId::One(window_id),
                        EventType::Mouse(event::MouseEvent::Pressed(button), local_x, local_y),
                    ));
                }
            }
            if status.released(button) {
                if button == 0 {
                    if manager.moving {
                        // debug!("id={window_id} Drag End");
                        manager.moving = false;
                    }
                }
                is_button_changed = true;
                debug!("id={window_id} Released");
                writer.push_event(Event::new(
                    DestId::One(window_id),
                    EventType::Mouse(event::MouseEvent::Released(button), local_x, local_y),
                ));
            }
        }
        if !is_button_changed {
            writer.push_event(Event::new(
                DestId::One(window_id),
                EventType::Mouse(event::MouseEvent::Move, local_x, local_y),
            ));
            if manager.moving {
                area = manager.top_layer().area().union(&area);
                manager.top_layer().move_(dx, dy);
            }
        }

        WINDOW_MANAGER
            .lock()
            .render_mouse(mouse_x, mouse_y, area, &mut get_graphic().lock());
    }
}

fn process_keyboard() {
    if let Some(key) = get_keystate_unblocked() {
        let id = WINDOW_MANAGER.lock().top_layer().window.lock().id;
        let event = Event::new(
            DestId::One(id),
            EventType::Keyboard(event::KeyEvent::Pressed(key)),
        );
        WINDOW_MANAGER.lock().top_layer().writer().push_event(event);
    }
}

pub fn window_task() {
    loop {
        process_mouse();
        process_keyboard();
    }
}
