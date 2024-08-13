pub mod apic;
pub mod asm;
mod handler;
mod idt;

use handler::*;
use idt::{EntryOptions, EntryTable};

use crate::{
    handler_with_context, handler_with_err_code, handler_without_err_code, sync::OnceLock,
};
use core::arch::asm;

pub use asm::{set_interrupt, without_interrupts};

#[repr(u8)]
pub enum InterruptVector {
    PATA1 = 0x2E,
    PATA2 = 0x2F,
    XHCI = 0x40,
    APICTimer = 0x41,
    IRQStart = 0x20,
}

static IDT: OnceLock<EntryTable> = OnceLock::new();

pub fn init_idt() {
    IDT.get_or_init(|| {
        let mut idt = EntryTable::new();
        let option = EntryOptions::new().set_dpl(0).set_stack_index(1);

        for i in 0..16 {
            idt.set_handler(i, handler_without_err_code!(common_exception))
                .set_option(option);
        }

        for i in 0..16 {
            idt.set_handler(
                i + InterruptVector::IRQStart as u8,
                handler_without_err_code!(irq_dummy_handler),
            )
            .set_option(option);
        }

        idt.set_handler(0, handler_without_err_code!(divided_by_zero))
            .set_option(option);
        idt.set_handler(3, handler_without_err_code!(break_point))
            .set_option(option);
        idt.set_handler(6, handler_without_err_code!(invalid_opcode))
            .set_option(option);
        // idt.set_handler(7, handler_with_err_code!(device_not_available))
        //     .set_option(option);
        idt.set_handler(8, handler_with_err_code!(double_fault))
            .set_option(option);
        idt.set_handler(13, handler_with_err_code!(general_protection))
            .set_option(option);
        idt.set_handler(14, handler_with_err_code!(page_fault))
            .set_option(option);

        idt.set_handler(
            InterruptVector::PATA1 as u8,
            handler_without_err_code!(pata1_handler),
        )
        .set_option(option);
        idt.set_handler(
            InterruptVector::PATA2 as u8,
            handler_without_err_code!(pata2_handler),
        )
        .set_option(option);

        idt.set_handler(
            InterruptVector::XHCI as u8,
            handler_without_err_code!(xhc_handler),
        )
        .set_option(option);

        idt.set_handler(
            InterruptVector::APICTimer as u8,
            handler_with_context!(apic_timer_handler),
        )
        .set_option(option);

        idt
    });

    IDT.load();
}
