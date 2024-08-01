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
            msi::{Message, Msi},
            search::{Base, Interface, PciSearcher, Sub},
            switch_ehci_to_xhci, Pci, PciDevice,
        },
        xhc::{self, allocator::Allocator, regist_controller, register},
    },
    entry_point,
    font::write_ascii,
    gdt::init_gdt,
    graphic::{graphic, GraphicWriter, PixelColor},
    interrupt::{
        apic::{APICTimerMode, LocalAPICId, LocalAPICRegisters},
        init_idt, set_interrupt, without_interrupts, InterruptVector,
    },
    page::init_page,
    print, println,
    task::{create_task, init_task, schedule},
};
use log::{debug, info, trace, warn};

entry_point!(kernel_main);

fn kernel_main(boot_info: BootInfo) {
    set_interrupt(false);
    let (height, width) = boot_info.frame_config.resolution();

    let pixel_writer = graphic(boot_info.frame_config);
    pixel_writer.clean();

    init_console(pixel_writer, PixelColor::Black, PixelColor::White);

    info!("Rust Kernel Started");

    init_gdt();
    info!("GDT Initialized");

    init_idt();
    set_interrupt(true);
    info!("IDT Initialized");

    init_page();
    info!("Page Table Initialized");

    init_heap(&boot_info.memory_map);
    info!("Heap Initialized");

    init_task();
    info!("Task Management Initialized");

    info!("ACPI Initialize info");
    if !boot_info.rsdp.is_valid() {
        info!("RSDP Validation Failed");
    } else {
        info!("RSDP Validation Success");
    }

    LocalAPICRegisters::default().apic_timer().init(
        0b1011,
        false,
        APICTimerMode::Periodic,
        InterruptVector::APICTimer as u8,
    );

    info!("Enable APIC Timer Interrupt");
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
        Some(xhc_dev) => {
            info!(
                "xHC has been found: {}.{}.{}",
                xhc_dev.bus, xhc_dev.dev, xhc_dev.func
            );
            let xhc_bar = xhc_dev.read_bar(0);
            info!("Read BAR0: 0x{xhc_bar:016X}");
            let xhc_mmio_base = xhc_bar & (!0xfu64);
            info!("xHC MMIO base: 0x{xhc_mmio_base:016X}");

            if xhc_dev.read_vendor_id() == 0x8086 {
                switch_ehci_to_xhci(&xhc_dev);
            }

            without_interrupts(|| {
                let lapic_id = LocalAPICRegisters::default().local_apic_id().id();
                let msg = Message::new()
                    .destionation_id(lapic_id)
                    .interrupt_index(InterruptVector::XHCI as u8)
                    .level(true)
                    .trigger_mode(true)
                    .delivery_mode(0);
                xhc_dev.capabilities().for_each(|cap| {
                    debug!("Capability ID={:?}", cap.id());
                    if let Some(msi) = cap.msi() {
                        debug!("MSI Initialize Start");
                        msi.enable(&msg);
                        debug!("MSI Initialize Success");
                    } else if let Some(msi) = cap.msix() {
                        debug!("MSI-X Initialize Start");
                        msi.enable(&msg);
                        debug!("MSI-X Initialize Success");
                    }
                });

                let mut allocator = Allocator::new();
                let keyboard = Keyboard::new();
                let mut xhc: xhc::Controller<register::External, Allocator> =
                    xhc::Controller::new(xhc_mmio_base, allocator, vec![Box::new(keyboard.usb())])
                        .unwrap();
                xhc.reset_port().expect("xHCI Port Reset Failed");
                regist_controller(xhc);
            });
            create_task(0, task1 as u64);
        }
        None => {}
    }
    create_task(0, task2 as u64);
    create_task(0, task3 as u64);
}

fn task1() {
    loop {
        print!("{}", getch() as char);
    }
}

fn task2() {
    loop {
        print!("a");
    }
}

fn task3() {
    loop {
        print!("b");
    }
}
