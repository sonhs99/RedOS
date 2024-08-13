use keycode::{Key, KeySpecial};
use log::debug;
use usb::USBKeyboardDriver;

use crate::{interrupt::without_interrupts, print, queue::ArrayQueue, sync::Mutex, task::schedule};

mod keycode;
mod manager;
mod usb;

const KEYBOARD_BUFFER_LENGTH: usize = 200;

static QUEUE: Mutex<ArrayQueue<Key, KEYBOARD_BUFFER_LENGTH>> =
    Mutex::new(ArrayQueue::new(Key::Special(KeySpecial::None)));

pub struct Keyboard {}

impl Keyboard {
    pub fn new() -> Self {
        Self {}
    }

    pub fn usb(&self) -> USBKeyboardDriver {
        USBKeyboardDriver::new(|byte, key| unsafe {
            without_interrupts(|| QUEUE.lock().enqueue(key));
        })
    }
}

pub fn get_code() -> Key {
    while QUEUE.lock().is_empty() {
        schedule();
    }
    without_interrupts(|| QUEUE.lock().dequeue().unwrap())
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
