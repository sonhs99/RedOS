use core::arch::asm;

pub fn set_interrupt(flag: bool) -> bool {
    let rflags: u64;
    unsafe {
        asm!("
        pushfq
        pop rax
        ", out("rax") rflags);
        if flag {
            asm!("sti");
        } else {
            asm!("cli");
        }
    }
    rflags & 0x0200 != 0
}

pub fn without_interrupts<F, T>(mut inner: F) -> T
where
    F: FnMut() -> T,
{
    let flag = set_interrupt(false);
    let result = inner();
    set_interrupt(flag);
    result
}
