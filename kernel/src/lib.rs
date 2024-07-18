#![no_std]
#![no_main]
#![feature(lazy_cell)]
#![feature(generic_arg_infer)]
#![feature(generic_nonzero)]
#![feature(naked_functions)]

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
pub mod task;

use core::panic::PanicInfo;
use log::error;

#[macro_export]
macro_rules! entry_point {
    ($path:path) => {
        const _: () = {
            const KERNEL_STACK: KernelStack = KernelStack::new();

            #[repr(C, align(16))]
            struct KernelStack([u8; 0x100000]);

            impl KernelStack {
                #[inline(always)]
                const fn new() -> Self {
                    Self([0; 0x100000])
                }

                #[inline(always)]
                pub fn end_addr(&self) -> u64 {
                    self.0.as_ptr() as u64 + 0x200000
                }
            }

            #[export_name = "_start"]
            pub unsafe extern "sysv64" fn __impl_start(boot_info: BootInfo) -> ! {
                use core::arch::asm;
                let f: fn(BootInfo) = $path;
                let stack_start_addr = KERNEL_STACK.end_addr();

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
    error!("{}", info);
    loop {}
}
