use alloc::rc::Rc;
use log::debug;

use crate::device::xhc::driver::{ClassDriverOperate, DriverType};

trait MouseSubscriber {
    fn subscribe(&self, pressed: u8, released: u8, x_v: i8, y_v: i8, z_v: i8);
}

impl<F> MouseSubscriber for F
where
    F: Fn(u8, u8, i8, i8, i8),
{
    fn subscribe(&self, pressed: u8, released: u8, x_v: i8, y_v: i8, z_v: i8) {
        self(pressed, released, x_v, y_v, z_v)
    }
}

#[derive(Clone)]
pub struct USBMouseDriver {
    prev_state: u8,
    buffer: [i8; 4],
    subscribe: Rc<dyn MouseSubscriber>,
}

impl USBMouseDriver {
    pub fn new<F>(subscribe: F) -> Self
    where
        F: Fn(u8, u8, i8, i8, i8) + 'static,
    {
        Self {
            prev_state: 0,
            buffer: [0i8; 4],
            subscribe: Rc::new(subscribe),
        }
    }
}

impl ClassDriverOperate for USBMouseDriver {
    fn on_data_received(&mut self) -> Result<(), ()> {
        // debug!("Mouse");
        let state = self.buffer[0] as u8;
        let pressed = !self.prev_state & state;
        let released = self.prev_state & !state;
        self.subscribe.subscribe(
            pressed,
            released,
            self.buffer[1],
            self.buffer[2],
            self.buffer[3],
        );
        self.prev_state = state;
        Ok(())
    }

    fn data_buffer_addr(&self) -> u64 {
        self.buffer.as_ptr() as u64
    }

    fn data_buffer_len(&self) -> u32 {
        4
    }

    fn driver_type(&self) -> DriverType {
        DriverType::Mouse
    }
}
