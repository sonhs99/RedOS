use core::hint::spin_loop;

use crate::{
    acpi::{FADT, FADT_CELL},
    device::Port,
    sync::OnceLock,
    task::schedule,
};

const PM_TIMER_FREQUENCY: u32 = 3579545;
static PM_TIMER: OnceLock<Port> = OnceLock::new();

pub fn init() {
    PM_TIMER.get_or_init(|| Port::new(FADT_CELL.timer() as u16));
}

#[inline]
pub fn read_count() -> u32 {
    PM_TIMER.in32()
}

#[inline]
pub fn convert_ms_to_tick(ms: u32) -> u32 {
    (ms * PM_TIMER_FREQUENCY) / 1000
}

#[inline]
pub fn convert_us_to_tick(us: u32) -> u32 {
    (us * PM_TIMER_FREQUENCY) / (1000 * 1000)
}
