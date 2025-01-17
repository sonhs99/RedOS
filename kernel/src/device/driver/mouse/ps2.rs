use alloc::rc::Rc;
use log::debug;

trait MouseSubscriber {
    fn subscribe(&self, pressed: u8, released: u8, x_v: i16, y_v: i16);
}

impl<F> MouseSubscriber for F
where
    F: Fn(u8, u8, i16, i16),
{
    fn subscribe(&self, pressed: u8, released: u8, x_v: i16, y_v: i16) {
        self(pressed, released, x_v, y_v)
    }
}

pub struct PS2MouseDriver {
    count: usize,
    state: u8,
    x_v: u8,
    y_v: u8,
    z_v: u8,
    prev_state: u8,
    subscribe: Rc<dyn MouseSubscriber>,
}

impl PS2MouseDriver {
    pub fn new<F>(subscribe: F) -> Self
    where
        F: Fn(u8, u8, i16, i16) + 'static,
    {
        Self {
            count: 0,
            state: 0,
            x_v: 0,
            y_v: 0,
            z_v: 0,
            prev_state: 0,
            subscribe: Rc::new(subscribe),
        }
    }

    pub fn on_data_received(&mut self, data: u8) {
        match self.count {
            0 => {
                self.state = data;
                self.count += 1;
            }
            1 => {
                self.x_v = data;
                self.count += 1;
            }
            2 => {
                self.y_v = data;
                self.count += 1;
            }
            _ => {
                self.count = 0;
            }
        }
        if self.count >= 3 {
            let curr_state = self.state & 0x07;
            let mut x_v = self.x_v as i16;
            let mut y_v = self.y_v as i16;
            if self.state & 0x10 != 0 {
                x_v -= 256;
            }
            if self.state & 0x20 != 0 {
                y_v -= 256;
            }
            y_v = -y_v;

            let pressed = curr_state & !self.prev_state;
            let released = !curr_state & self.prev_state;
            self.prev_state = curr_state;

            // debug!("Mouse status={:08b} x_v={}, y_v={}", self.state, x_v, y_v);
            self.subscribe
                .subscribe(pressed & 0x07, released & 0x07, x_v, y_v);
            self.count = 0;
        }
    }
}
