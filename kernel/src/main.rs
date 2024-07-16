#![no_std]
#![no_main]

extern crate alloc;

use alloc::{boxed::Box, vec};
use core::arch::asm;

use bootloader::{BootInfo, FrameBufferConfig, PixelFormat};
use kernel::{
    allocator::init_heap,
    console::{init_console, Console},
    device::{
        driver::keyboard::{getch, Keyboard},
        pci::{
            init_pci,
            search::{Base, Interface, PciSearcher, Sub},
            switch_ehci_to_xhci, Pci, PciDevice,
        },
        xhc::{self, allocator::Allocator, register},
    },
    entry_point,
    font::write_ascii,
    gdt::init_gdt,
    graphic::{graphic, GraphicWriter, PixelColor},
    interrupt::init_idt,
    page::init_page,
    print, println,
};
use log::{debug, info, trace, warn};

entry_point!(kernel_main);

fn kernel_main(boot_info: BootInfo) {
    let (height, width) = boot_info.frame_config.resolution();

    let pixel_writer = graphic(boot_info.frame_config);
    pixel_writer.clean();

    init_console(pixel_writer, PixelColor::Black, PixelColor::White);

    info!("Rust Kernel Started");

    init_gdt();
    info!("GDT Initialized");

    init_idt();
    info!("IDT Initialized");
    debug!("Interrupt Test");
    unsafe { asm!("int 3") };
    debug!("Interrupt Test Success");

    init_page();
    info!("Page Table Initialized");

    init_heap(&boot_info.memory_map);
    info!("Heap Initialized");

    info!("PCI Init Started");
    init_pci();

    match PciSearcher::new()
        .base(Base::Serial)
        .sub(Sub::USB)
        .interface(Interface::XHCI)
        .search()
        .expect("No xHC device detected")
        .first()
    {
        Some(dev) => {
            info!("xHC has been found: {}.{}.{}", dev.bus, dev.dev, dev.func);
            let xhc_bar = dev.read_bar(0);
            info!("Read BAR0: 0x{xhc_bar:016X}");
            let xhc_mmio_base = xhc_bar & (!0xfu64);
            info!("xHC MMIO base: 0x{xhc_mmio_base:016X}");

            if dev.read_vendor_id() == 0x8086 {
                switch_ehci_to_xhci(&dev);
            }

            let mut allocator = Allocator::new();
            let keyboard = Keyboard::new();
            let mut xhc: xhc::Controller<register::External, Allocator> =
                xhc::Controller::new(xhc_mmio_base, allocator, vec![Box::new(keyboard.usb())])
                    .unwrap();
            xhc.reset_port().expect("xHCI Port Reset Failed");
            info!("xHCI Pooling Start");

            xhc.start_event_pooling();
        }
        None => {}
    }
}
