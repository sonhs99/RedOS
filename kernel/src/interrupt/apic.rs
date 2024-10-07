use core::ptr::{read_volatile, write_volatile};

use log::debug;

use crate::{percpu::lapic_register_base, sync::OnceLock, timer::wait_ms};

pub struct LocalAPICRegisters(u32);

impl LocalAPICRegisters {
    pub fn new(base: u32) -> Self {
        Self(base)
    }

    pub fn end_of_interrupt(&self) -> EndOfInterrupt {
        EndOfInterrupt(self.0 + 0xB0)
    }

    pub fn local_apic_id(&self) -> LocalAPICId {
        LocalAPICId(self.0 + 0x20)
    }

    pub fn apic_timer(&self) -> APICTimer {
        APICTimer(self.0)
    }

    pub fn int_control(&self) -> LocalAPICIntCommand {
        LocalAPICIntCommand::new(self.0)
    }

    pub fn error(&self) -> LocalAPICError {
        LocalAPICError::new(self.0)
    }

    pub fn svr(&self) -> LocalAPICSVR {
        LocalAPICSVR::new(self.0)
    }
}

impl Default for LocalAPICRegisters {
    fn default() -> Self {
        Self(lapic_register_base())
    }
}

pub struct EndOfInterrupt(u32);

impl EndOfInterrupt {
    pub fn notify(&self) {
        unsafe { write_volatile(self.0 as *mut u32, 0) };
    }
}

pub struct LocalAPICId(u32);

impl LocalAPICId {
    pub fn id(&self) -> u8 {
        unsafe { (read_volatile(self.0 as *const u32) >> 24) as u8 }
    }
}

#[repr(u8)]
pub enum APICTimerMode {
    OneShot = 0,
    Periodic = 1,
}

static TICK_PER_100MILISECOND: OnceLock<u32> = OnceLock::new();
const TICK_DIVIDER: u32 = 1000;
pub struct APICTimer(u32);

impl APICTimer {
    const LVT_TIMER: u32 = 0x320;
    const INIT_COUNTER: u32 = 0x380;
    const CURRENT_COUNTER: u32 = 0x390;
    const DIVIDER: u32 = 0x3E0;

    const INIT_COUNTER_VALUE: u32 = 0xFFFF_FFFF;

    pub fn init(&self, divider: u8, mask: bool, mode: APICTimerMode, vector: u8) {
        let mut data = vector as u32;
        data |= (mask as u32) << 16;
        data |= (mode as u32) << 17;
        self.start();
        wait_ms(100);
        let elapse = self.elapsed();
        TICK_PER_100MILISECOND.get_or_init(|| elapse);
        // debug!("APIC Timer: {elapse} Tick/100ms");
        unsafe {
            write_volatile((self.0 + Self::DIVIDER) as *mut u32, divider as u32);
            write_volatile((self.0 + Self::LVT_TIMER) as *mut u32, data);
            write_volatile(
                (self.0 + Self::INIT_COUNTER) as *mut u32,
                elapse / TICK_DIVIDER,
            );
        }
    }

    pub fn start(&self) {
        unsafe {
            write_volatile(
                (self.0 + Self::INIT_COUNTER) as *mut u32,
                Self::INIT_COUNTER_VALUE,
            )
        };
    }

    pub fn elapsed(&self) -> u32 {
        unsafe {
            Self::INIT_COUNTER_VALUE - read_volatile((self.0 + Self::CURRENT_COUNTER) as *const u32)
        }
    }

    pub fn tick_count() -> u32 {
        TICK_PER_100MILISECOND.get().unwrap() / TICK_DIVIDER
    }
}

pub struct IOAPICRegister(u32);

impl IOAPICRegister {
    pub fn new(address: u32) -> Self {
        Self(address)
    }

    pub fn read(&self, address: u8) -> u32 {
        unsafe {
            write_volatile(self.0 as *mut u32, address as u32);
            read_volatile((self.0 + 0x10) as *const u32)
        }
    }

    pub fn write(&self, address: u8, value: u32) {
        unsafe {
            write_volatile(self.0 as *mut u32, address as u32);
            write_volatile((self.0 + 0x10) as *mut u32, value);
        }
    }
}

impl Default for IOAPICRegister {
    fn default() -> Self {
        Self(0xFEC0_0000)
    }
}

pub struct LocalAPICIntCommand {
    low: u32,
    high: u32,
}

impl LocalAPICIntCommand {
    pub fn new(address: u32) -> Self {
        Self {
            low: address + 0x300,
            high: address + 0x310,
        }
    }

    pub fn write(&self, low_val: u32, high_val: u32) {
        unsafe {
            write_volatile(self.high as *mut u32, high_val);
            write_volatile(self.low as *mut u32, low_val);
        }
    }

    pub fn read(&self) -> u32 {
        unsafe { read_volatile(self.low as *const u32) }
    }
}

impl Default for LocalAPICIntCommand {
    fn default() -> Self {
        Self {
            low: 0xFEE0_0300,
            high: 0xFEE0_0310,
        }
    }
}

pub struct LocalAPICError(u32);

impl LocalAPICError {
    pub fn new(address: u32) -> Self {
        Self(0xFEE0_0280)
    }

    pub fn read(&self) -> u32 {
        unsafe { read_volatile(self.0 as *const u32) }
    }
}

pub struct LocalAPICSVR(u32);

impl LocalAPICSVR {
    pub fn new(address: u32) -> Self {
        Self(address + 0x0F0)
    }

    pub fn write(&self, value: u32) {
        unsafe { write_volatile(self.0 as *mut u32, value) };
    }

    pub fn read(&self) -> u32 {
        unsafe { read_volatile(self.0 as *const u32) }
    }
}
