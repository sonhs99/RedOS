#![no_std]
#![no_main]
#![feature(lazy_cell)]
#![feature(generic_arg_infer)]

pub mod console;
pub mod device;
pub mod font;
pub mod graphic;
pub mod sync;

use bootloader::{BootInfo, FrameBufferConfig, PixelFormat};
use core::panic::PanicInfo;
use log::error;

#[macro_export]
macro_rules! entry_point {
    ($path:path) => {
        const _: () = {
            #[export_name = "_start"]
            pub extern "C" fn __impl_start(boot_info: BootInfo) -> ! {
                let f: fn(BootInfo) = $path;
                f(boot_info);
                loop {
                    unsafe { asm!("hlt") };
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
