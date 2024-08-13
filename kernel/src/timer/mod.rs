use crate::{
    acpi::{FADT, FADT_CELL},
    device::Port,
    sync::OnceLock,
    task::schedule,
};

const PM_TIMER_FREQUENCY: u32 = 3579545;
static PM_TIMER: OnceLock<Port> = OnceLock::new();

pub fn init_pm() {
    PM_TIMER.get_or_init(|| Port::new(FADT_CELL.timer() as u16));
}

#[inline]
pub fn read_pm_count() -> u32 {
    PM_TIMER.in32()
}

#[inline]
pub fn convert_ms_to_tick(ms: u32) -> u32 {
    ms * PM_TIMER_FREQUENCY / 1000
}

#[inline]
pub fn convert_us_to_tick(us: u32) -> u32 {
    us * PM_TIMER_FREQUENCY / (1000 * 1000)
}

pub fn wait_ms(ms: u32) {
    wait_tick(convert_ms_to_tick(ms));
}

pub fn wait_us(us: u32) {
    wait_tick(convert_us_to_tick(us));
}

pub fn wait_tick(tick: u32) {
    let start = read_pm_count();
    while read_pm_count().wrapping_sub(start) <= tick {
        // schedule();
    }
}

pub fn sleep(ms: u32) {
    let start = read_pm_count();
    let range = convert_ms_to_tick(ms);
    // if FADT_CELL.flags() & 0x10 == 0 {
    //     end &= 0x00FF_FFFF;
    // }
    while read_pm_count().wrapping_sub(start) <= range {
        schedule();
    }
}
