#![no_std]
#![no_main]

extern crate alloc;

use alloc::{boxed::Box, vec};
use core::arch::asm;

use bootloader::{BootInfo, FrameBufferConfig, PixelFormat};
use kernel::{
    acpi,
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
    float::set_ts,
    font::write_ascii,
    gdt::init_gdt,
    graphic::{get_graphic, init_graphic, GraphicWriter, PixelColor, PIXEL_WRITER},
    interrupt::{
        apic::{APICTimerMode, LocalAPICId, LocalAPICRegisters},
        init_idt, set_interrupt, without_interrupts, InterruptVector,
    },
    page::init_page,
    print, println,
    task::{create_task, exit, idle, init_task, running_task, schedule, TaskFlags},
};
use log::{debug, info, trace, warn};

entry_point!(kernel_main);

fn kernel_main(boot_info: BootInfo) {
    set_interrupt(false);
    let (height, width) = boot_info.frame_config.resolution();

    init_graphic(boot_info.frame_config);
    get_graphic().lock().clean();

    init_console(PixelColor::Black, PixelColor::White);

    info!("Rust Kernel Started");

    init_gdt();
    info!("GDT Initialized");

    init_idt();
    info!("IDT Initialized");

    init_page();
    info!("Page Table Initialized");

    init_heap(&boot_info.memory_map);
    info!("Heap Initialized");

    init_task();
    info!("Task Management Initialized");

    // Do Not Use
    // set_ts();
    // info!("Lazy FP Enable");

    acpi::initialize(boot_info.rsdp);
    info!("ACPI Initialized");

    LocalAPICRegisters::default().apic_timer().init(
        0b1011,
        false,
        APICTimerMode::Periodic,
        InterruptVector::APICTimer as u8,
    );
    set_interrupt(true);

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
            create_task(TaskFlags::new(), print_input as u64, 0, 0);
        }
        None => {}
    }
    info!("{}/{} = {}", width, height, width as f64 / height as f64);
    create_task(TaskFlags::new(), test as u64, 0, 0);
}

fn print_input() {
    loop {
        print!("{}", getch() as char);
    }
}

fn test() {
    for i in 0..50 {
        create_task(TaskFlags::new().thread().clone(), test_thread as u64, 0, 0);
        // create_task(TaskFlags::new().thread().clone(), test_fpu as u64, 0, 0);
        // create_task(
        //     TaskFlags::new().thread().clone(),
        //     test_windmill as u64,
        //     0,
        //     0,
        // );
        // info!("Thread {i} created");
    }
    for i in 0..100 {
        create_task(
            TaskFlags::new().thread().set_priority(66).clone(),
            test_thread as u64,
            0,
            0,
        );
    }
    loop {}
}

fn test_fpu() {
    let id = running_task().id() + 1;
    let mut count = 1.0f64;

    for i in 0..10 {
        let before = count;
        let factor = (id + i) as f64 / id as f64;
        count *= factor;
        // info!("PID={:3}| count(mul)={:.6}", id, count);
        count /= factor;
        // info!("PID={:3}| count(div)={:.6}", id, count);
        if before != count {
            info!(
                "PID={:3}| Test Failed, before={:.6}, after={:.6}",
                id, before, count
            );
            return;
        }
    }
    info!("PID={:3}| Test Success", id);
}

fn test_thread() {
    let id = running_task().id() + 1;
    let mut random = id;
    let mut value1 = 1f64;
    let mut value2 = 1f64;

    let data = [b'-', b'\\', b'|', b'/'];
    let offset = id * 2;
    let offset_x = id % 80 + 80;
    let offset_y = id / 80 + 25;
    let mut count = 0;

    loop {
        random = random * 1103515245 + 12345;
        random = (random >> 16) & 0xFFFF_FFFF;
        let factor = random % 255;
        let factor = (factor + id) as f64 / id as f64;
        value1 *= factor;
        value2 *= factor;

        if value1 != value2 {
            break;
        }

        value1 /= factor;
        value2 /= factor;

        if value1 != value2 {
            break;
        }

        write_ascii(
            offset_x * 8,
            offset_y * 16,
            data[count],
            PixelColor::Red,
            PixelColor::Black,
        );
        count = (count + 1) % 4;
    }
    write_ascii(
        offset_x * 8,
        offset_y * 16,
        b' ',
        PixelColor::Red,
        PixelColor::White,
    );
    info!("Thread id={id}: FPU Test Failed -> left={value1}, right={value2}");
}

fn test_windmill() {
    let id = running_task().id() + 1;
    let data = [b'-', b'\\', b'|', b'/'];
    let offset = id * 2;
    let offset_x = id % 80 + 80;
    let offset_y = id / 80 + 25;
    let mut count = 0;

    loop {
        write_ascii(
            offset_x * 8,
            offset_y * 16,
            data[count],
            PixelColor::Red,
            PixelColor::Black,
        );
        count = (count + 1) % 4;
    }
}
