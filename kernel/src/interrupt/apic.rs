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
