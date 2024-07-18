use core::ptr::{read_volatile, write_volatile};

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
}

impl Default for LocalAPICRegisters {
    fn default() -> Self {
        Self(0xFEE0_0000)
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

pub struct APICTimer(u32);

impl APICTimer {
    const LVT_TIMER: u32 = 0x320;
    const INIT_COUNTER: u32 = 0x380;
    const CURRENT_COUNTER: u32 = 0x390;
    const DIVIDER: u32 = 0x3E0;

    const INIT_COUNTER_VALUE: u32 = 0x0100_0000;
    pub fn init(&self, divider: u8, mask: bool, mode: APICTimerMode, vector: u8) {
        let mut data = vector as u32;
        data |= (mask as u32) << 16;
        data |= (mode as u32) << 17;
        unsafe {
            write_volatile((self.0 + Self::DIVIDER) as *mut u32, divider as u32);
            write_volatile((self.0 + Self::LVT_TIMER) as *mut u32, data);
            write_volatile(
                (self.0 + Self::INIT_COUNTER) as *mut u32,
                Self::INIT_COUNTER_VALUE,
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
}
