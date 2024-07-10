#![no_std]
#![no_main]
#![feature(lazy_cell)]
#![feature(generic_arg_infer)]
#![feature(generic_nonzero)]

extern crate alloc;

pub mod allocator;
pub mod console;
pub mod device;
pub mod font;
pub mod gdt;
pub mod graphic;
pub mod interrupt;
pub mod page;
mod queue;
pub mod sync;

use core::panic::PanicInfo;
use log::error;

#[macro_export]
macro_rules! entry_point {
    ($path:path) => {
        const _: () = {
            // #[repr(align(16))]
            // struct KernelStack([u8; 1024 * 1024]);

            // static KERNEL_STACK: KernelStack = KernelStack([0u8; 1024 * 1024]);
            #[export_name = "_start"]
            pub unsafe extern "C" fn __impl_start(boot_info: BootInfo) -> ! {
                // let boot = boot_info.clone();
                let f: fn(BootInfo) = $path;
                // asm!("mov rsp, {}", in(reg) &KERNEL_STACK.0[1024 * 1024 - 1] as *const u8 as u64);
                // asm!("mov rbp, {}", in(reg) &KERNEL_STACK.0[1024 * 1024 - 1] as *const u8 as u64);
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
    error!("{}", info);
    loop {}
}
