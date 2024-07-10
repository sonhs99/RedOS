use log::debug;

use super::keycode::{Key, KeySpecial};

#[derive(Clone)]
pub struct KeyboardManager {
    capslock: bool,
    numlock: bool,
    scrolllock: bool,
}

impl KeyboardManager {
    pub const fn new() -> Self {
        KeyboardManager {
            capslock: false,
            numlock: false,
            scrolllock: false,
        }
    }

    pub const fn is_combined_code(&self, keycode: u8, shift_pressed: bool) -> bool {
        if Self::is_alpha(keycode) {
            self.capslock ^ shift_pressed
        // } else if Self::is_num(keycode) {
        //     shift_pressed
        } else if Self::is_num_pad(keycode) {
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

    const fn is_num(keycode: u8) -> bool {
        keycode >= 0x1E && keycode <= 0x27
    }

    const fn is_num_pad(keycode: u8) -> bool {
        keycode >= 0x59 && keycode <= 0x63
    }

    const fn is_alpha(keycode: u8) -> bool {
        keycode >= 0x04 && keycode <= 0x1D
    }
}
