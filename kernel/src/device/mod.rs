pub mod driver;
pub mod hdd;
pub mod pci;
// pub mod ps2;
pub mod xhc;

use core::arch::asm;

pub struct Port {
    port: u16,
}

impl Port {
    pub const fn new(port: u16) -> Port {
        Port { port }
    }

    pub fn out8(&self, data: u8) {
        unsafe {
            asm!(
                "out dx, al",
                in("dx") self.port,
                in("al") data,
                options(nostack, preserves_flags))
        };
    }

    pub fn in8(&self) -> u8 {
        let mut output: u8;
        unsafe {
            asm!(
                "mov rax, 0",
                "in al, dx",
                in("dx") self.port,
                out("al") output,
                options(nostack, preserves_flags))
        };
        output
    }

    pub fn out16(&self, data: u16) {
        unsafe {
            asm!(
                "out dx, ax",
                in("dx") self.port,
                in("ax") data,
                options(nostack, preserves_flags))
        };
    }

    pub fn in16(&self) -> u16 {
        let mut output: u16;
        unsafe {
            asm!(
                "mov rax, 0",
                "in ax, dx",
                in("dx") self.port,
                out("ax") output,
                options(nostack, preserves_flags))
        };
        output
    }

    pub fn out32(&self, data: u32) {
        unsafe {
            asm!(
                "out dx, eax",
                in("dx") self.port,
                in("eax") data,
                options(nostack, preserves_flags))
        };
    }

    pub fn in32(&self) -> u32 {
        let mut output: u32;
        unsafe {
            asm!(
                "mov rax, 0",
                "in eax, dx",
                in("dx") self.port,
                out("eax") output,
                options(nostack, preserves_flags))
        };
        output
    }
}
