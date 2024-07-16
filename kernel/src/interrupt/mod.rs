mod handler;
mod idt;

use core::arch::asm;

use handler::*;
use idt::{EntryOptions, EntryTable};

use crate::{handler_with_err_code, handler_without_err_code, sync::OnceLock};

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

#[repr(u8)]
pub enum InterruptVector {
    NULL,
}

static IDT: OnceLock<EntryTable> = OnceLock::new();

pub fn init_idt() {
    IDT.get_or_init(|| {
        let mut idt = EntryTable::new();
        let option = EntryOptions::new().set_dpl(0).set_stack_index(0);

        idt.set_handler(0, handler_without_err_code!(divided_by_zero))
            .set_option(option);
        idt.set_handler(3, handler_without_err_code!(break_point))
            .set_option(option);
        idt.set_handler(6, handler_without_err_code!(invalid_opcode))
            .set_option(option);
        idt.set_handler(14, handler_with_err_code!(page_fault))
            .set_option(option);
        idt
    });

    IDT.load();
}
