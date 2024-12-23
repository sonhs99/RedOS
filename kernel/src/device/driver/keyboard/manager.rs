use log::debug;

use super::keycode::{Key, KeySpecial, Keycode};

#[derive(Clone)]
pub struct KeyboardManager {
    capslock: bool,
    numlock: bool,
    scrolllock: bool,
}

impl KeyboardManager {
    pub const fn new() -> Self {
        Self {
            capslock: false,
            numlock: false,
            scrolllock: false,
        }
    }

    pub fn is_combined_code(&self, keycode: impl Keycode, shift_pressed: bool) -> bool {
        if keycode.is_alpha() {
            self.capslock ^ shift_pressed
        } else if keycode.is_num_pad() {
            self.numlock
        } else {
            shift_pressed
        }
    }

    pub fn update_key_status(&mut self, key: &Key) {
        if let Key::Special(key_sp) = key {
            let led_status_change = match key_sp {
                KeySpecial::CapsLock => {
                    self.capslock ^= true;
                    true
                }
                KeySpecial::NumLock => {
                    self.numlock ^= true;
                    true
                }
                KeySpecial::ScrollLock => {
                    self.scrolllock ^= true;
                    true
                }
                _ => false,
            };

            // if led_status_change {
            //     change_keyboard_led(self.capslock, self.numlock, self.scrolllock);
            // }
        }
    }
}
