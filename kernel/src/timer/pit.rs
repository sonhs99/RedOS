use crate::device::Port;

const PIT_FREQUENCY: usize = 1193180;

const PIT_CONTROL_COUNTER0: u8 = 0x00;
const PIT_CONTROL_COUNTER1: u8 = 0x40;
const PIT_CONTROL_COUNTER2: u8 = 0x80;

const PIT_CONTROL_LATCH: u8 = 0x00;
const PIT_CONTROL_LOW: u8 = 0x10;
const PIT_CONTROL_HIGH: u8 = 0x20;
const PIT_CONTROL_LSBMSBRW: u8 = 0x30;

const PIT_CONTROL_INT: u8 = 0x00;
const PIT_CONTROL_CLOCK: u8 = 0x04;

const PIT_CONTROL_BCD: u8 = 0x01;
const PIT_CONTROL_BINARY: u8 = 0x00;

const PIT_CONTROL_ONCE: u8 = PIT_CONTROL_LSBMSBRW | PIT_CONTROL_INT | PIT_CONTROL_BINARY;
const PIT_CONTROL_PERIODIC: u8 = PIT_CONTROL_LSBMSBRW | PIT_CONTROL_CLOCK | PIT_CONTROL_BINARY;

static PIT_CONTROL: Port = Port::new(0x0043);
static PIT_COUNTER: [Port; 3] = [Port::new(0x0040), Port::new(0x0041), Port::new(0x0042)];

#[inline]
pub fn convert_ms_to_tick(ms: usize) -> u16 {
    ((ms * PIT_FREQUENCY) / 1000) as u16
}

#[inline]
pub fn convert_us_to_tick(us: usize) -> u16 {
    ((us * PIT_FREQUENCY) / (1000 * 1000)) as u16
}
pub fn init_counter(counter: u8, count: u16, periodic: bool) {
    let counter = match counter {
        0 => PIT_CONTROL_COUNTER0,
        1 => PIT_CONTROL_COUNTER1,
        2 => PIT_CONTROL_COUNTER2,
        _ => panic!("Not found PIT"),
    };
    let port = PIT_COUNTER.get(counter as usize).unwrap();
    let value = if periodic {
        PIT_CONTROL_PERIODIC
    } else {
        PIT_CONTROL_ONCE
    };
    PIT_CONTROL.out8(counter | value);
    port.out8(count as u8);
    port.out8((count >> 8) as u8);
}

pub fn read_counter(counter: u8) -> u16 {
    let port = PIT_COUNTER.get(counter as usize).expect("Not found PIT");
    let low = port.in8() as u16;
    let high = port.in8() as u16;
    low | high << 8
}
