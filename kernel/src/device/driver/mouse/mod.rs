use log::debug;
use ps2::PS2MouseDriver;
use usb::USBMouseDriver;

use crate::{
    collections::queue::Queue,
    sync::{Mark, Mutex},
    task::schedule,
};

pub mod ps2;
pub mod usb;

const MOUSE_BUFFER_LENGTH: usize = 200;
const MAX_SKIP_MOUSE_EVENT: usize = 20;

static QUEUE: Mark<Mutex<Queue<[MouseState; MOUSE_BUFFER_LENGTH]>>> = Mark::new(Mutex::new(
    Queue::new([MouseState::empty(); MOUSE_BUFFER_LENGTH]),
));

#[derive(Clone, Copy)]
pub struct MouseState {
    pressed: u8,
    released: u8,
    x_v: i16,
    y_v: i16,
    z_v: i16,
}

impl MouseState {
    pub const fn new(pressed: u8, released: u8, x_v: i16, y_v: i16, z_v: i16) -> Self {
        Self {
            pressed,
            released,
            x_v,
            y_v,
            z_v,
        }
    }
    pub const fn empty() -> Self {
        Self {
            pressed: 0,
            released: 0,
            x_v: 0,
            y_v: 0,
            z_v: 0,
        }
    }

    pub const fn pressed(&self, index: u8) -> bool {
        (self.pressed >> index) & 0x01 != 0
    }

    pub const fn released(&self, index: u8) -> bool {
        (self.released >> index) & 0x01 != 0
    }

    pub const fn x_v(&self) -> i16 {
        self.x_v
    }

    pub const fn y_v(&self) -> i16 {
        self.y_v
    }

    pub const fn z_v(&self) -> i16 {
        self.z_v
    }
}

pub struct Mouse {}

impl Mouse {
    pub fn new() -> Self {
        Self {}
    }

    pub fn usb(&self) -> USBMouseDriver {
        USBMouseDriver::new(|pressed, released, x_v, y_v, z_v| unsafe {
            // debug!("Mouse x_v={x_v}, y_v={y_v}, z_v={z_v}");
            let _ = QUEUE.skip().lock().enqueue(MouseState::new(
                pressed, released, x_v as i16, y_v as i16, z_v as i16,
            ));
        })
    }
    pub fn ps2(&self) -> PS2MouseDriver {
        PS2MouseDriver::new(|pressed, x_v, y_v| unsafe {
            // debug!("Mouse press={pressed:08b} x_v={x_v}, y_v={y_v}");
            let _ =
                QUEUE
                    .skip()
                    .lock()
                    .enqueue(MouseState::new(pressed, !pressed & 0x07, x_v, y_v, 0));
        })
    }
}

pub fn get_mouse_state() -> MouseState {
    let mut state = MouseState::empty();
    while QUEUE.skip().lock().is_empty() {
        schedule();
    }
    for _ in 0..MAX_SKIP_MOUSE_EVENT {
        let res = QUEUE.skip().lock().dequeue();
        match res {
            Ok(current) => {
                state.x_v += current.x_v;
                state.y_v += current.y_v;
                state.z_v += current.z_v;
                state.pressed |= current.pressed;
                state.released |= current.released;
            }
            Err(_) => break,
        }
    }
    // debug!(
    //     "dx={}, dy={}, pressed={:08b}, release={:08b}",
    //     state.x_v, state.y_v, state.pressed, state.released
    // );
    state
}

pub fn get_mouse_state_unblocked() -> Option<MouseState> {
    if QUEUE.skip().lock().is_empty() {
        None
    } else {
        let mut state = MouseState::empty();
        for _ in 0..MAX_SKIP_MOUSE_EVENT {
            let res = QUEUE.skip().lock().dequeue();
            match res {
                Ok(current) => {
                    state.x_v += current.x_v;
                    state.y_v += current.y_v;
                    state.z_v += current.z_v;
                    state.pressed |= current.pressed;
                    state.released |= current.released;
                }
                Err(_) => break,
            }
        }
        // debug!(
        //     "dx={}, dy={}, pressed={:08b}, release={:08b}",
        //     state.x_v, state.y_v, state.pressed, state.released
        // );
        Some(state)
    }
}
