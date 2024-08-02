#![no_std]
#![no_main]
#![feature(lazy_cell)]
#![feature(generic_arg_infer)]
#![feature(generic_nonzero)]
#![feature(naked_functions)]
#![feature(core_intrinsics)]

extern crate alloc;

pub mod acpi;
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

#[repr(C, align(16))]
pub struct KernelStack<const N: usize>([u8; N]);

impl<const N: usize> KernelStack<N> {
    #[inline(always)]
    pub const fn new() -> Self {
        Self([0; N])
    }

    #[inline(always)]
    pub fn end_addr(&self) -> u64 {
        self.0.as_ptr() as u64 + N as u64
    }

    #[inline(always)]
    pub fn start_addr(&self) -> u64 {
        self.0.as_ptr() as u64
    }

    #[inline(always)]
    pub fn size() -> usize {
        N
    }
}

#[macro_export]
macro_rules! entry_point {
    ($path:path) => {
        const _: () = {
            use kernel::KernelStack;
            const KERNEL_STACK: KernelStack<0x100000> = KernelStack::new();

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
