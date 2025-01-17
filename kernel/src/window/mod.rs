pub mod component;
mod curser;
pub mod draw;
pub mod event;
pub mod frame;
pub mod manager;
mod test;
pub mod writer;

use core::ops::DerefMut;

use alloc::fmt::format;
use alloc::format;
use alloc::vec::Vec;
use alloc::{sync::Arc, vec};
use draw::Point;
use event::{DestId, Event, EventType, MouseEvent, UpdateEvent, EVENT_QUEUE_SIZE};
// use hashbrown::HashMap;
use log::{debug, info};

use frame::WindowFrame;
use manager::WindowManager;
use test::test_window;
use writer::FrameBuffer;

use crate::collections::queue::Queue;
use crate::device::driver::keyboard::get_keystate_unblocked;
use crate::device::driver::mouse::get_mouse_state_unblocked;
use crate::task::{create_task, TaskFlags};
use crate::utility::abs;
use crate::{
    graphic::{get_graphic, GraphicWriter, PixelColor},
    sync::{Mutex, MutexGuard, OnceLock},
    utility::random,
};

pub use writer::WindowWriter;

const MAX_QUEUE_ENQUEUE_COUNT: usize = 40;

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
    fn write_id(&self) -> Option<usize>;
}

pub trait Movable {
    fn move_(&mut self, offset_x: isize, offset_y: isize);
    fn move_range(&mut self, offset_x: isize, offset_y: isize, area: Area);
}

pub trait BitmapDrawable: Drawable {
    fn bitmap_draw(
        &self,
        offset_x: usize,
        offset_y: usize,
        area: &Area,
        bitmap: &mut DrawBitmap,
        writer: &mut impl Writable,
    );
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

    pub const fn width(&self) -> usize {
        self.width
    }

    pub const fn height(&self) -> usize {
        self.height
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

struct DrawBitmap {
    area: Area,
    bitmap: Vec<u8>,
}

impl DrawBitmap {
    pub fn new(area: Area) -> Self {
        let size = (area.size() + 7) / 8;
        Self {
            area,
            bitmap: vec![0u8; size],
        }
    }

    pub fn mark_area(&mut self, area: &Area) {
        if let Some(conjointed) = area.conjoint(&self.area) {
            let width = self.area.width;
            let offset_area = conjointed.offset_of(&self.area);
            for y in offset_area.y..(offset_area.y + offset_area.height) {
                let offset = y * width;
                let start = offset_area.x + offset;
                let end = offset_area.x + offset_area.width + offset;
                for idx in (start + 7) / 8..(end - 1) / 8 {
                    self.bitmap[idx] = 0xFF;
                }
                let remain = 8 - start % 8;
                self.bitmap[start / 8] |= 0xFFu8.wrapping_shr(remain as u32);
                let remain = 8 - end % 8;
                self.bitmap[(end - 1) / 8] |= 0xFFu8.wrapping_shl(remain as u32);
            }
        }
    }

    pub fn point(&self, point: Point) -> bool {
        if self.area.is_in(point.0, point.1) {
            let local = Point(point.0 - self.area.x, point.1 - self.area.y);
            let idx = local.0 + local.1 * self.area.width;
            let offset = idx % 8;
            (self.bitmap[idx / 8] >> (7 - offset)) & 0x01 == 0
        } else {
            false
        }
    }

    pub fn set_point(&mut self, point: Point) {
        if self.area.is_in(point.0, point.1) {
            let local = Point(point.0 - self.area.x, point.1 - self.area.y);
            let idx = local.0 + local.1 * self.area.width;
            let offset = idx % 8;
            self.bitmap[idx / 8] |= 0x01 << (7 - offset);
        }
    }
}

pub struct Window {
    id: usize,
    width: usize,
    height: usize,
    transparent_color: Option<u32>,
    buffer: Vec<u32>,
    event_queue: Queue<Vec<Event>>,
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
            event_queue: Queue::new(vec![Event::default(); EVENT_QUEUE_SIZE]),
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
}

impl Drawable for Window {
    fn draw(&self, offset_x: usize, offset_y: usize, area: &Area, writer: &mut impl Writable) {
        for (y, chunk) in self.buffer[area.y * self.width..(area.y + area.height) * self.width]
            .chunks(self.width)
            .enumerate()
        {
            if let Some(transparent) = self.transparent_color {
                for x in area.x..area.x + area.width {
                    if chunk[x] != transparent {
                        writer.write(
                            offset_x + area.x + x,
                            offset_y + area.y + y,
                            chunk[x].into(),
                        );
                    }
                }
            } else {
                writer.write_buf(
                    offset_x + area.x,
                    offset_y + area.y + y,
                    &chunk[area.x..area.x + area.width],
                );
            }
        }
    }
}

impl BitmapDrawable for Window {
    fn bitmap_draw(
        &self,
        offset_x: usize,
        offset_y: usize,
        area: &Area,
        bitmap: &mut DrawBitmap,
        writer: &mut impl Writable,
    ) {
        for (y, chunk) in self.buffer[area.y * self.width..(area.y + area.height) * self.width]
            .chunks(self.width)
            .enumerate()
        {
            for x in area.x..area.x + area.width {
                let global_x = offset_x + area.x + x;
                let global_y = offset_y + area.y + y;
                if bitmap.point(Point(global_x, global_y)) {
                    writer.write(global_x, global_y, chunk[x].into());
                    // bitmap.set_point(Point(global_x, global_y));
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
    update: bool,
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
            update: true,
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
        self.need_update();
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
        let flag = self.update;
        self.update = false;
        flag
    }

    pub fn need_update(&mut self) {
        self.update = true;
    }

    pub fn writer(&mut self) -> WindowWriter {
        WindowWriter(self.window.clone())
    }
}

impl Drawable for Layer {
    fn draw(&self, offset_x: usize, offset_y: usize, area: &Area, writer: &mut impl Writable) {
        let my_area = self.area();
        if let Some(conjointed_area) = self.area().conjoint(&my_area) {
            let offset_area = conjointed_area.offset_of(&my_area);
            self.window
                .lock()
                .draw(offset_x + self.x, offset_y + self.y, &offset_area, writer);
        }
    }
}

impl BitmapDrawable for Layer {
    fn bitmap_draw(
        &self,
        offset_x: usize,
        offset_y: usize,
        area: &Area,
        bitmap: &mut DrawBitmap,
        writer: &mut impl Writable,
    ) {
        let my_area = self.area();
        if let Some(conjointed_area) = self.area().conjoint(&my_area) {
            let offset_area = conjointed_area.offset_of(&my_area);
            self.window.lock().bitmap_draw(
                offset_x + self.x,
                offset_y + self.y,
                &offset_area,
                bitmap,
                writer,
            );
        }
        bitmap.mark_area(&my_area);
    }
}

impl Writable for GraphicWriter {
    fn write(&mut self, x: usize, y: usize, color: PixelColor) {
        GraphicWriter::write(&self, x, y, color);
    }

    fn write_buf(&mut self, offset_x: usize, offset_y: usize, buffer: &[u32]) {
        GraphicWriter::write_buf(&self, offset_x, offset_y, buffer);
    }

    fn write_id(&self) -> Option<usize> {
        None
    }
}

impl<T: Writable> Writable for MutexGuard<'_, T> {
    fn write(&mut self, x: usize, y: usize, color: PixelColor) {
        self.deref_mut().write(x, y, color);
    }

    fn write_buf(&mut self, offset_x: usize, offset_y: usize, buffer: &[u32]) {
        self.deref_mut().write_buf(offset_x, offset_y, buffer);
    }

    fn write_id(&self) -> Option<usize> {
        None
    }
}

static WINDOW_MANAGER: OnceLock<Mutex<WindowManager>> = OnceLock::new();

pub fn init_window(resolution: (usize, usize)) {
    WINDOW_MANAGER.get_or_init(|| Mutex::new(WindowManager::new(resolution)));

    let mut manager = WINDOW_MANAGER.lock();
    let bg_layer = manager.new_layer_pos(0, 0, resolution.0, resolution.1, false);
    let mut writer = bg_layer.writer();
    let buffer = vec![PixelColor(232, 255, 232).as_u32(); resolution.0];
    for y in 0..resolution.1 {
        writer.write_buf(0, y, &buffer);
    }

    // let nav_bar = manager.new_layer_pos(0, 0, resolution.0, 18, false);
    // let mut writer = nav_bar.writer();
    // let buffer = vec![PixelColor::Black.as_u32(); resolution.0];
    // for y in 0..18 {
    //     writer.write_buf(0, y, &buffer);
    // }
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

pub fn request_update_by_id(id: usize) {
    WINDOW_MANAGER.lock().push_event(Event::new(
        DestId::One(id),
        EventType::Update(event::UpdateEvent::Id(id)),
    ));
}

pub fn request_update_by_area(area: Area) {
    WINDOW_MANAGER.lock().push_event(Event::new(
        DestId::None,
        EventType::Update(event::UpdateEvent::Area(area)),
    ));
}

pub fn request_update_all_windows() {
    WINDOW_MANAGER.lock().push_event(Event::new(
        DestId::All,
        EventType::Update(event::UpdateEvent::All),
    ));
}

fn process_mouse() -> bool {
    if let Some(status) = get_mouse_state_unblocked() {
        // debug!("asdf");
        let mut manager = WINDOW_MANAGER.lock();

        let resolution = manager.resolution();
        let prev = manager.get_mouse();
        let mut area = Area::new(prev.0, prev.1, 16, 24);
        let mouse_x =
            (prev.0 as isize + status.x_v() as isize).clamp(0, resolution.0 as isize - 1) as usize;
        let mouse_y =
            (prev.1 as isize + status.y_v() as isize).clamp(0, resolution.1 as isize - 1) as usize;
        let new_area = Area::new(mouse_x, mouse_y, 16, 24);

        let window_id = manager.get_layer_id_from_point(mouse_x, mouse_y);
        let layer = manager.get_layer(window_id).unwrap();

        let (local_x, local_y) = layer.area().local(mouse_x, mouse_y);
        let writer = layer.writer();
        let mut is_button_changed = false;

        let dx = mouse_x as isize - prev.0 as isize;
        let dy = mouse_y as isize - prev.1 as isize;

        for button in 0..8 {
            if status.pressed(button) {
                let changed = if button == 0 {
                    manager.focus(window_id);
                    request_update_by_id(window_id);
                    match writer.get_area(local_x, local_y) {
                        WindowComponent::Title => {
                            is_button_changed = true;
                            writer.push_event(Event::new(
                                DestId::One(window_id),
                                EventType::Window(event::WindowEvent::Move),
                            ));
                            // debug!("id={window_id} Drag Start");
                            area = manager.top_layer().area().union(&area);
                            manager.top_layer().move_(dx, dy);
                            manager.set_moving(true);
                            true
                        }
                        WindowComponent::Close => {
                            is_button_changed = true;
                            writer.push_event(Event::new(
                                DestId::One(window_id),
                                EventType::Window(event::WindowEvent::Close),
                            ));
                            // debug!("[WINDOW] Window Remove Start");
                            true
                        }
                        WindowComponent::Body => false,
                    }
                } else {
                    false
                };
                if !changed {
                    // debug!("id={window_id} Pressed");
                    is_button_changed = true;

                    writer.push_event(Event::new(
                        DestId::One(window_id),
                        EventType::Mouse(event::MouseEvent::Pressed(button), local_x, local_y),
                    ));
                }
                if window_id == 0 && button == 0 {
                    create_task(
                        "WindowTest",
                        TaskFlags::new().set_priority(66).clone(),
                        None,
                        test_window as u64,
                        0,
                        0,
                    );
                }
            }
            if status.released(button) {
                if button == 0 {
                    if manager.moving() {
                        // debug!("id={window_id} Drag End");
                        manager.set_moving(false);
                    }
                }
                is_button_changed = true;
                let top_writer = manager.top_layer().writer();
                let top_id = top_writer.write_id().unwrap();
                // debug!("id={window_id} Released");
                top_writer.push_event(Event::new(
                    DestId::One(top_id),
                    EventType::Mouse(event::MouseEvent::Released(button), local_x, local_y),
                ));
            }
        }
        if !is_button_changed {
            writer.push_event(Event::new(
                DestId::One(window_id),
                EventType::Mouse(event::MouseEvent::Move, local_x, local_y),
            ));
            if manager.moving() {
                area = manager.top_layer().area().union(&area);
                manager.top_layer().move_(dx, dy);
            }
        }

        // WINDOW_MANAGER
        //     .lock()
        //     .render_mouse(mouse_x, mouse_y, area, &mut get_graphic().lock());
        WINDOW_MANAGER.lock().set_mouse(Point(mouse_x, mouse_y));
        request_update_by_area(new_area);
        request_update_by_area(area);
        // debug!("[WINDOW] x={mouse_x} y={mouse_y}");
        // request_update_all_windows();
        return true;
    }
    false
}

fn process_keyboard() -> bool {
    if let Some(key) = get_keystate_unblocked() {
        let id = WINDOW_MANAGER.lock().top_layer().window.lock().id;
        let event = Event::new(
            DestId::One(id),
            EventType::Keyboard(event::KeyEvent::Pressed(key)),
        );
        WINDOW_MANAGER.lock().top_layer().writer().push_event(event);
        return true;
    }
    false
}

fn process_window() {
    let mut update = false;
    let mut global_area = None;
    for _ in 0..MAX_QUEUE_ENQUEUE_COUNT {
        let result = WINDOW_MANAGER.lock().pop_event();
        if let Ok(event) = result {
            if let EventType::Update(update_event) = event.event() {
                update = true;
                match update_event {
                    UpdateEvent::Id(id) => {
                        if let Some(layer) = WINDOW_MANAGER.lock().get_layer(id) {
                            layer.need_update();
                            let area = layer.area();
                            global_area = if let Some(a) = global_area {
                                Some(area.union(&a))
                            } else {
                                Some(area)
                            };
                        }
                    }
                    UpdateEvent::Area(area) => {
                        global_area = if let Some(a) = global_area {
                            Some(area.union(&a))
                        } else {
                            Some(area)
                        };
                    }
                    UpdateEvent::All => {
                        let mut manager = WINDOW_MANAGER.lock();
                        // let area = manager.area();
                        manager.request_global_update();
                        global_area = Some(manager.area());
                    }
                }
            }
        } else {
            break;
        }
    }
    if update {
        let mut manager = WINDOW_MANAGER.lock();
        let area = global_area.unwrap_or(manager.area());
        WINDOW_MANAGER
            .lock()
            .render(&area, &mut get_graphic().lock());
    }
}

pub fn window_task() {
    let mut count = 0;
    loop {
        let mouse = process_mouse();
        let keyboard = process_keyboard();
        process_window();
        if mouse || keyboard {
            // debug!("Mouse={mouse}, Keyboard={keyboard}");
            count += 1;
        }
    }
}
