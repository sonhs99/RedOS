#![no_std]
#![no_main]

extern crate alloc;

use alloc::vec;
use alloc::{boxed::Box, format, string::String, vec::Vec};
use core::{arch::asm, iter::empty, ptr::read_volatile, str};
use kernel::console::alloc_window;
use kernel::device::driver::mouse::{get_mouse_state, Mouse};
use kernel::device::ps2::keyboard::init_ps2;
use kernel::shell::start_shell;
use kernel::task::idle::idle_task;
use kernel::window::component::Button;
use kernel::window::draw::{draw_rect, draw_str, Point};
use kernel::window::event::{EventType, WindowEvent};
use kernel::window::frame::WindowFrame;
use kernel::window::{
    create_window, init_window, request_update_all_windows, window_task, Drawable, Writable,
};

use bootloader::{BootInfo, FrameBufferConfig, PixelFormat};
use kernel::{
    acpi,
    allocator::init_heap,
    ap::init_ap,
    console::{init_console, Console},
    device::{
        block::{
            pata::{get_device, init_pata},
            ram::RamDisk,
            Block,
        },
        driver::keyboard::{get_code, getch, Keyboard},
        pci::{
            init_pci,
            msi::{init_routing_table, MSIEntry, Message, Msi},
            search::{Base, Interface, PciSearcher, Sub},
            switch_ehci_to_xhci, Pci, PciDevice,
        },
        xhc::{self, allocator::Allocator, regist_controller, register},
    },
    entry_point,
    font::write_ascii,
    fs::{self, dev_list, flush, format_by_name, init_fs, mount, open, open_dir},
    gdt::init_gdt,
    graphic::{get_graphic, init_graphic, GraphicWriter, PixelColor, PIXEL_WRITER},
    interrupt::{
        apic::{APICTimerMode, IOAPICRegister, LocalAPICId, LocalAPICRegisters},
        init_idt, set_interrupt, without_interrupts, InterruptVector,
    },
    ioapic,
    page::init_page,
    percpu, print, println,
    task::{create_task, exit, idle, init_task, running_task, schedule, TaskFlags},
    timer::init_pm,
};
use log::{debug, error, info, trace, warn};

entry_point!(kernel_main);

fn kernel_main(boot_info: BootInfo) {
    set_interrupt(false);
    let (width, height) = boot_info.frame_config.resolution();

    init_graphic(boot_info.frame_config);
    get_graphic().lock().clean();

    init_console(PixelColor::Black, PixelColor::White);

    info!("Rust Kernel Started");

    init_gdt(boot_info.ist_frame.0, boot_info.ist_frame.1 as u64);
    info!("GDT Initialized");

    init_idt();
    info!("IDT Initialized");

    init_page();
    info!("Page Table Initialized");

    init_heap(&boot_info.memory_map);
    info!("Heap Initialized");

    init_task();
    info!("Task Management Initialized");

    init_fs();
    info!("Root File System Initialized");

    let ramdisk = RamDisk::new(8 * 1024 * 1024);
    mount(ramdisk, "ram0", false);
    format_by_name("ram0", 8 * 1024 * 1024, false);
    info!("RAM Disk Mounted");

    // Do Not Use
    // set_ts();
    // info!("Lazy FP Enable");

    acpi::initialize(boot_info.rsdp);
    info!("ACPI Initialized");

    let num_core = ioapic::init();
    percpu::init(num_core, 0xFEE0_0000);
    info!("I/O APIC Initialized");
    info!("Number Of Core: {num_core}");

    init_pm();
    info!("ACPI PM Timer Initialized");

    LocalAPICRegisters::default().apic_timer().init(
        0b1011,
        false,
        APICTimerMode::Periodic,
        InterruptVector::APICTimer as u8,
    );
    info!("Enable APIC Timer Interrupt");
    set_interrupt(true);

    if boot_info.ap_bootstrap.is_some() {
        let wake_up_count = init_ap(
            num_core,
            boot_info.stack_frame.0,
            boot_info.stack_frame.1 as u64,
        )
        .expect("AP Wakeup Failed");
        info!("{wake_up_count} AP Wakeup");
    }

    // loop {}

    info!("Device Init Start");

    let keyboard = Keyboard::new();
    let mouse = Mouse::new();

    init_ps2(keyboard.ps2(), mouse.ps2());
    info!("Enable PS/2 Keyboard and Mouse");

    info!("PCI Init Started");
    init_pci();

    // create_task(TaskFlags::new(), None, test as u64, 0, 0);

    let mut msi_vector: Vec<MSIEntry> = Vec::new();
    if let Some(xhci) = PciSearcher::new()
        .base(Base::Serial)
        .sub(Sub::USB)
        .interface(Interface::XHCI)
        .search()
    {
        for (idx, xhc_dev) in xhci.iter().enumerate() {
            info!(
                "[{}] xHC has been found: {}.{}.{}",
                idx, xhc_dev.bus, xhc_dev.dev, xhc_dev.func
            );
            let xhc_bar = xhc_dev.read_bar(0);
            info!("Read BAR0: 0x{xhc_bar:016X}");
            let xhc_mmio_base = xhc_bar & (!0xfu64);
            info!("xHC MMIO base: 0x{xhc_mmio_base:016X}");

            if xhc_dev.read_vendor_id() == 0x8086 {
                switch_ehci_to_xhci(&xhc_dev);
            }

            without_interrupts(|| {
                let msg = Message::new()
                    .destionation_id(0x00)
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
                        msi_vector.push(MSIEntry::MSI(msi.device.clone(), msi.offset));
                    } else if let Some(msi) = cap.msix() {
                        debug!("MSI-X Initialize Start");
                        msi.enable(&msg);
                        debug!("MSI-X Initialize Success");
                        msi_vector.push(MSIEntry::MSIX(msi.table(), 0));
                    }
                });

                let mut allocator = Allocator::new();
                let mut xhc: xhc::Controller<register::External, Allocator> = xhc::Controller::new(
                    xhc_mmio_base,
                    allocator,
                    vec![Box::new(keyboard.usb()), Box::new(mouse.usb())],
                )
                .unwrap();
                xhc.reset_port().expect("xHCI Port Reset Failed");
                regist_controller(xhc);
            });
        }
    } else {
        info!("No xHC Device Found");
    }

    if let Some(ide) = PciSearcher::new()
        .base(Base::MassStorage)
        .sub(Sub::IDE)
        .interface(Interface::None)
        .search()
    {
        for (idx, ide_dev) in ide.iter().enumerate() {
            info!(
                "IDE has been found: {}.{}.{}",
                ide_dev.bus, ide_dev.dev, ide_dev.func
            );
            init_pata();
            for i in 0..4 {
                if let Ok(hdd) = get_device(i) {
                    info!("PATA:{i} Detected");
                    // // create_task(TaskFlags::new(), test_hdd as u64, 0, 0);
                    let dev_name = format!("pata{i}");
                    if let Ok(fs_count) = mount(hdd, &dev_name, true) {
                        info!("PATA:{i} mounted, fs_count={fs_count}");
                        if fs_count == 0 {
                            if let Err(reason) =
                                format_by_name(&dev_name, 1024 * 1024 * 10 / 512, true)
                            {
                                info!("PATA:{i} format failed");
                                info!("{}", reason);
                            } else {
                                info!("PATA:{i} formated");
                            }
                        }

                        // let root = open_dir(&dev_name, 0, "/", b"r")
                        //     .expect("Attempt to Open Root Directory Failed");
                        // let mut count = 0;
                        // for (idx, entry) in root.entries() {
                        //     info!("[{idx}] /{entry}");
                        //     count += 1;
                        // }
                        // info!("Total {count} entries");
                    }
                } else {
                    info!("PATA:{i} Not Detected");
                }
            }
            // info!("List of Block I/O Device");
            // for (idx, dev_name) in dev_list().iter().enumerate() {
            //     info!("[{idx}] {dev_name}");
            // }
            // create_task(TaskFlags::new(), None, test_fs as u64, 0, 0);
        }
    } else {
        info!("No IDE Device Found");
    }

    init_routing_table(msi_vector);

    init_window((width, height));
    create_task("window", TaskFlags::new(), None, window_task as u64, 0, 0);
    let mut writer = WindowFrame::new_pos(0, 19, 640, 400, "Log");
    writer.set_background(PixelColor::Black);
    alloc_window(writer);
    create_task("shell", TaskFlags::new(), None, start_shell as u64, 0, 0);
    request_update_all_windows();
    idle_task();
}
