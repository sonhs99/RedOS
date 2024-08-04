use core::arch::asm;

pub fn fpu_init() {
    unsafe { asm!("finit", options(nostack)) };
}

pub fn fpu_save(buffer: u64) {
    unsafe { asm!("fxsave [rdi]", in("rdi") buffer, options(nostack)) };
}

pub fn fpu_load(buffer: u64) {
    unsafe { asm!("fxrstor [rdi]", in("rdi") buffer, options(nostack)) };
}

pub fn set_ts() {
    unsafe { asm!("mov rax, cr0", "or rax, 0x08", "mov cr0, rax") };
}

pub fn clear_ts() {
    unsafe { asm!("clts", options(nostack)) };
}
