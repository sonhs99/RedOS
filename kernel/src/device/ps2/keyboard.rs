use log::debug;

use crate::{
    device::driver::{keyboard::ps2::PS2KeyboardDriver, mouse::ps2::PS2MouseDriver},
    interrupt::without_interrupts,
    sync::{Mutex, OnceLock},
};

use super::super::Port;

pub static KEYBOARD: OnceLock<Mutex<Keyboard>> = OnceLock::new();

pub struct Keyboard {
    control: Port,
    state: Port,
    input_buffer: Port,
    output_buffer: Port,
    keyboard_driver: PS2KeyboardDriver,
    mouse_driver: PS2MouseDriver,
}

impl Keyboard {
    pub const fn new(
        control: u16,
        state: u16,
        input_buffer: u16,
        output_buffer: u16,
        keyboard_driver: PS2KeyboardDriver,
        mouse_driver: PS2MouseDriver,
    ) -> Self {
        Keyboard {
            control: Port::new(control),
            state: Port::new(state),
            input_buffer: Port::new(input_buffer),
            output_buffer: Port::new(output_buffer),
            keyboard_driver,
            mouse_driver,
        }
    }

    fn is_mouse_data_in_buffer(&self) -> bool {
        (self.state.in8() & 0x20) != 0
    }

    fn is_input_buffer_full(&self) -> bool {
        (self.state.in8() & 0x02) != 0
    }

    fn is_output_buffer_full(&self) -> bool {
        (self.state.in8() & 0x01) != 0
    }

    fn wait_ack_and_key_code(&mut self) -> Result<(), ()> {
        for _ in 0..100 {
            for _ in 0..0xFFFF {
                if self.is_output_buffer_full() {
                    break;
                }
            }
            let is_mouse = self.is_mouse_data_in_buffer();
            let data = self.output_buffer.in8();
            if data == 0xFA {
                return Ok(());
            } else if is_mouse {
                self.mouse_driver.on_data_received(data);
            } else {
                self.keyboard_driver.on_data_received(data);
            }
        }
        Err(())
    }

    pub fn activate_keyboard(&mut self) -> Result<(), ()> {
        self.control.out8(0xAE);
        for _ in 0..0xFFFF {
            if !self.is_input_buffer_full() {
                break;
            }
        }

        self.input_buffer.out8(0xF4);
        self.wait_ack_and_key_code()
    }

    pub fn activate_mouse(&mut self) -> Result<(), ()> {
        self.control.out8(0xA8);
        self.control.out8(0xD4);
        for _ in 0..0xFFFF {
            if !self.is_input_buffer_full() {
                break;
            }
        }

        self.input_buffer.out8(0xF4);
        self.wait_ack_and_key_code()
    }

    pub fn get_data(&mut self) {
        if self.is_output_buffer_full() {
            if self.is_mouse_data_in_buffer() {
                let data = self.output_buffer.in8();
                // debug!("mouse {data:02X}");
                self.mouse_driver.on_data_received(data);
            } else {
                let data = self.output_buffer.in8();
                // debug!("keyboard {data:02X}");
                self.keyboard_driver.on_data_received(data);
            }
        }
    }

    pub fn enable_a20_gate(&self) {
        self.control.out8(0xD0);
        for _ in 0..0xFFFF {
            if self.is_output_buffer_full() {
                break;
            }
        }
        let data = self.output_buffer.in8();
        let data = data | 0x01;
        for _ in 0..0xFFFF {
            if !self.is_input_buffer_full() {
                break;
            }
        }
        self.control.out8(0xD1);
        self.input_buffer.out8(data);
    }

    pub fn change_led(&mut self, capslock: bool, numlock: bool, scrolllock: bool) {
        for _ in 0..0xFFFF {
            if !self.is_input_buffer_full() {
                break;
            }
        }
        self.output_buffer.out8(0xED);
        for _ in 0..0xFFFF {
            if !self.is_input_buffer_full() {
                break;
            }
        }
        let _ = self.wait_ack_and_key_code();
        self.output_buffer
            .out8((capslock as u8) << 2 | (numlock as u8) << 1 | scrolllock as u8);
        for _ in 0..0xFFFF {
            if !self.is_input_buffer_full() {
                break;
            }
        }
        let _ = self.wait_ack_and_key_code();
    }
}

pub fn init_ps2(keyboard_driver: PS2KeyboardDriver, mouse_driver: PS2MouseDriver) {
    let mut keyboard = KEYBOARD
        .get_or_init(|| {
            Mutex::new(Keyboard::new(
                0x64,
                0x64,
                0x60,
                0x60,
                keyboard_driver,
                mouse_driver,
            ))
        })
        .lock();
    without_interrupts(|| {
        keyboard.activate_keyboard();
        keyboard.activate_mouse();
    });
}

pub fn change_keyboard_led(capslock: bool, numlock: bool, scrolllock: bool) {
    KEYBOARD.lock().change_led(capslock, numlock, scrolllock);
}
