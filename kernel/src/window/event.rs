use crate::device::driver::{keyboard::keycode::Key, mouse::MouseState};

use super::Area;

pub const EVENT_QUEUE_SIZE: usize = 20;

#[derive(Clone, Copy)]
pub enum DestId {
    One(usize),
    All,
    None,
}

#[derive(Clone, Copy)]
pub enum EventType {
    Mouse(MouseEvent, usize, usize),
    Window(WindowEvent),
    Keyboard(KeyEvent),
    Update(UpdateEvent),
    Custom(CustomEvent),
    Unknown,
}

#[derive(Clone, Copy, Debug)]
pub enum MouseEvent {
    Move,
    Pressed(u8),
    Released(u8),
}

#[derive(Clone, Copy)]
pub enum WindowEvent {
    Select,
    Released,
    Move,
    Close,
}

#[derive(Clone, Copy)]
pub enum KeyEvent {
    Pressed(Key),
    Released(Key),
}

#[derive(Clone, Copy)]
pub enum UpdateEvent {
    Id(usize),
    Area(Area),
    // Remove(usize),
    All,
}

#[derive(Clone, Copy)]
pub struct CustomEvent([u64; 3]);

#[derive(Clone, Copy)]
pub struct Event {
    dest: DestId,
    event: EventType,
}

impl Event {
    pub const fn new(dest: DestId, event: EventType) -> Self {
        Self { dest, event }
    }

    pub const fn event(&self) -> EventType {
        self.event
    }

    pub const fn dest(&self) -> DestId {
        self.dest
    }
}

impl Default for Event {
    fn default() -> Self {
        Self {
            dest: DestId::None,
            event: EventType::Unknown,
        }
    }
}
