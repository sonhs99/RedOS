mod handler;

use core::arch::asm;

pub fn cli() {
    unsafe { asm!("cli") };
}

pub fn sti() {
    unsafe { asm!("sti") };
}

pub fn without_interrupt<F, T>(mut inner: F) -> T
where
    F: FnMut() -> T,
{
    cli();
    let result = inner();
    sti();
    result
}
