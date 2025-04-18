#![no_std]
#![no_main]
#![feature(naked_functions)]
#![feature(core_intrinsics)]
#![feature(allocator_api)]
#![allow(warnings)]
#![feature(is_none_or)]

extern crate alloc;

pub mod acpi;
pub mod allocator;
pub mod ap;
pub mod cache;
mod collections;
pub mod console;
pub mod device;
pub mod float;
pub mod font;
pub mod fs;
pub mod gdt;
pub mod graphic;
pub mod interrupt;
pub mod ioapic;
pub mod page;
pub mod percpu;
pub mod shell;
mod sync;
pub mod task;
// pub mod tests;
pub mod timer;
pub mod utility;
pub mod window;

use core::panic::PanicInfo;
use interrupt::apic::LocalAPICRegisters;
use log::error;
use task::{exit, running_task};

#[macro_export]
macro_rules! entry_point {
    ($path:path) => {
        const _: () = {
            #[export_name = "_start"]
            pub unsafe extern "sysv64" fn __impl_start(boot_info: BootInfo) -> ! {
                use core::arch::asm;
                let f: fn(BootInfo) = $path;
                let stack_start_addr = boot_info.stack_frame.0 + boot_info.stack_frame.1 as u64;

        asm!("mov rsp, {0}", in(reg) stack_start_addr);
                f(boot_info);

                loop {
                    asm!("hlt");
                }
            }
        };
    };
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    let apic_id = LocalAPICRegisters::default().local_apic_id().id();
    if let Some(running) = running_task() {
        panic_print!(
            "Core={},PID={},in {}\n{}",
            apic_id,
            running.id(),
            running.name(),
            info
        );
        if !running.flags().is_system_task() {
            exit();
        }
    } else {
        panic_print!("Core={}\n{}", apic_id, info);
    }
    loop {}
}
