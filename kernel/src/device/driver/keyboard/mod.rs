use keycode::{Key, KeySpecial};
use log::debug;
use usb::USBKeyboardDriver;

use crate::{
    collections::queue::Queue, interrupt::without_interrupts, print, sync::Mutex, task::schedule,
};

pub mod keycode;
mod manager;
mod usb;

const KEYBOARD_BUFFER_LENGTH: usize = 200;

static QUEUE: Mutex<Queue<[Key; KEYBOARD_BUFFER_LENGTH]>> = Mutex::new(Queue::new(
    [Key::Special(KeySpecial::None); KEYBOARD_BUFFER_LENGTH],
));

pub struct Keyboard {}

impl Keyboard {
    pub fn new() -> Self {
        Self {}
    }

    pub fn usb(&self) -> USBKeyboardDriver {
        USBKeyboardDriver::new(|byte, key| unsafe {
            QUEUE.lock().enqueue(key);
        })
    }
}

pub fn get_keystate_unblocked() -> Option<Key> {
    QUEUE.lock().dequeue().ok()
}

pub fn get_code() -> Key {
    while QUEUE.lock().is_empty() {
        schedule();
    }
    QUEUE.lock().dequeue().unwrap()
}

pub fn getch() -> u8 {
    while true {
        if let Key::Ascii(byte) = get_code() {
            return byte;
        }
        schedule();
    }
    0u8
}
