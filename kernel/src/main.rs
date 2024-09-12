#![no_std]
#![no_main]

extern crate alloc;

use alloc::vec;
use alloc::{boxed::Box, format, string::String, vec::Vec};
use core::{arch::asm, iter::empty, ptr::read_volatile, str};
use kernel::console::alloc_window;
use kernel::device::driver::mouse::{get_mouse_state, Mouse};
use kernel::task::idle::idle_task;
use kernel::window::component::{write_str, Rectangle};
use kernel::window::event::{EventType, WindowEvent};
use kernel::window::frame::WindowFrame;
use kernel::window::{create_window, init_window, window_task, Drawable};

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
    float::set_ts,
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

    init_window((width, height));
    let writer = WindowFrame::new_pos(0, 19, 640, 400, "Log");
    alloc_window(writer);
    info!("Window Manager Initialized");

    init_task();
    create_task(TaskFlags::new(), None, window_task as u64, 0, 0);
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
    percpu::init(num_core);
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

    info!("PCI Init Started");
    init_pci();

    // create_task(TaskFlags::new(), None, test as u64, 0, 0);

    let mut msi_vector: Vec<MSIEntry> = Vec::new();
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
                let keyboard = Keyboard::new();
                let mouse = Mouse::new();
                let mut xhc: xhc::Controller<register::External, Allocator> = xhc::Controller::new(
                    xhc_mmio_base,
                    allocator,
                    vec![Box::new(keyboard.usb()), Box::new(mouse.usb())],
                )
                .unwrap();
                xhc.reset_port().expect("xHCI Port Reset Failed");
                regist_controller(xhc);
            });
            // create_task(TaskFlags::new(), None, print_input as u64, 0, 0);
        }
        None => {}
    }

    match PciSearcher::new()
        .base(Base::MassStorage)
        .sub(Sub::IDE)
        .interface(Interface::None)
        .search()
        .expect("No IDE device detected")
        .first()
    {
        Some(ide_dev) => {
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

                        let root = open_dir(&dev_name, 0, "/", b"r")
                            .expect("Attempt to Open Root Directory Failed");
                        let mut count = 0;
                        for (idx, entry) in root.entries() {
                            info!("[{idx}] /{entry}");
                            count += 1;
                        }
                        info!("Total {count} entries");
                    }
                } else {
                    info!("PATA:{i} Not Detected");
                }
            }
            info!("List of Block I/O Device");
            for (idx, dev_name) in dev_list().iter().enumerate() {
                info!("[{idx}] {dev_name}");
            }
            // create_task(TaskFlags::new(), None, test_fs as u64, 0, 0);
        }
        None => {}
    }

    init_routing_table(msi_vector);
    create_task(TaskFlags::new(), None, test_window as u64, 0, 0);
    idle_task();
}

fn print_input() {
    let mut count = 0usize;
    loop {
        print!("{}", getch() as char);
        count = count.wrapping_add(1);
        // if count % 10 == 0 {
        //     WindowFrame::new(200, 200, "Test");
        // }
    }
}

fn test_hdd() {
    let mut buffer: [Block<512>; 1] = [const { Block::empty() }; 1];
    let mut hdd = get_device(1).expect("Cannot find HDD");
    info!("PATA HDD Test Start");
    info!("1. Read");
    for lba in 0..4 {
        hdd.read_block(lba, &mut buffer).expect("HDD Read Failed");
        for (lba_offset, block) in buffer.iter().enumerate() {
            for idx in 0..512 {
                if idx % 16 == 0 {
                    print!(
                        "\nLBA={:2X}, offset={:3X}    |",
                        lba + lba_offset as u32,
                        idx
                    )
                }
                print!("{:02X} ", block.get::<u8>(idx));
            }
            println!();
        }
    }

    // info!("2. Write");
    // for block in buffer.iter_mut() {
    //     for idx in 0..512 {
    //         *block.get_mut(idx) = idx as u8;
    //     }
    // }
    // write_block(1, 0, &buffer).expect("HDD Write Failed");
    // read_block(1, 0, &mut buffer).expect("HDD Read Failed");
    // for (lba, block) in buffer.iter().enumerate() {
    //     for idx in 0..512 {
    //         if idx % 16 == 0 {
    //             print!("\nLBA={:2X}, offset={:3X}    |", lba, idx)
    //         }
    //         print!("{:02X} ", block.get::<u8>(idx));
    //     }
    //     println!();
    // }
}

fn test_window() {
    let width = 500;
    let height = 200;
    let mut writer = WindowFrame::new(width, height, "Hello, World!");
    let mut info = writer.info();
    let id = writer.window_id();
    let rect = Rectangle::new(
        width - 20,
        70,
        2,
        0,
        PixelColor::White,
        PixelColor::Black,
        PixelColor::White,
    );
    rect.draw(10, 8, &rect.outside_pos(10, 8), &mut info);
    write_str(
        20,
        4,
        &format!("GUI Information Window[Window ID: {id:#08X}]"),
        PixelColor::Black,
        PixelColor::White,
        &mut info,
    );

    write_str(
        16,
        24,
        "Mouse Event:",
        PixelColor::Black,
        PixelColor::White,
        &mut info,
    );
    write_str(
        16,
        40,
        "Data: X = 0, Y = 0",
        PixelColor::Black,
        PixelColor::White,
        &mut info,
    );

    loop {
        if let Some(event) = writer.pop_event() {
            match event.event() {
                EventType::Mouse(e, x, y) => {
                    let str = match e {
                        kernel::window::event::MouseEvent::Move => "Move",
                        kernel::window::event::MouseEvent::Pressed(_) => "Pressed",
                        kernel::window::event::MouseEvent::Released(_) => "Released",
                    };
                    write_str(
                        16,
                        24,
                        &format!("Mouse Event: {str:10}"),
                        PixelColor::Black,
                        PixelColor::White,
                        &mut info,
                    );
                    write_str(
                        16,
                        40,
                        &format!("Data: X = {x:3}, Y = {y:3}"),
                        PixelColor::Black,
                        PixelColor::White,
                        &mut info,
                    );
                }
                EventType::Window(e) => {
                    if let WindowEvent::Close = e {
                        writer.close();
                        return;
                    }
                }
                _ => {}
            }
        }
    }
}

fn test_hdd_rw() {
    let mut buffer: [Block<512>; 1] = [Block::empty(); 1];
    let mut pattern: [[Block<512>; 1]; 4] = [const { [Block::empty(); 1] }; 4];

    let hdd = get_device(1).expect("Cannot find HDD");

    for block in pattern[0].iter_mut() {
        for idx in 0..512 {
            *block.get_mut(idx) = idx as u8;
        }
    }

    for block in pattern[1].iter_mut() {
        for idx in 0..512 {
            *block.get_mut(idx) = (idx as u8) % 16;
        }
    }

    for block in pattern[2].iter_mut() {
        for idx in 0..512 {
            *block.get_mut(idx) = (idx as u8) % 2;
        }
    }

    for block in pattern[3].iter_mut() {
        for idx in 0..512 {
            if idx % 4 == 0 {
                *block.get_mut(idx) = 1;
            }
        }
    }

    let mut flag = false;
    info!("PATA HDD Read/Write Test Start");
    for (lba, pattern_buffer) in pattern.iter().enumerate() {
        info!("Pattern {}", lba + 1);
        hdd.write_block(lba as u32, pattern_buffer)
            .expect("HDD Write Failed");
        hdd.read_block(lba as u32, &mut buffer)
            .expect("HDD Read Failed");
        for idx in 0..512 {
            if *pattern_buffer[0].get::<u8>(idx) != *buffer[0].get::<u8>(idx) {
                flag = true;
                break;
            }
        }
        if flag {
            error!("Test Failed");
            for (pattern_block, block) in pattern_buffer.iter().zip(buffer.iter()) {
                for idx in 0..512 * 2 {
                    let offset = idx & 0x0F | (idx & !0x1F) >> 1;
                    if idx % 16 == 0 {
                        if idx % 32 == 0 {
                            print!("\nLBA={:2X}, offset={:3X}  |", lba as u32, idx)
                        } else {
                            print!(" |  ")
                        }
                    }
                    if (idx >> 4) & 0x01 == 0 {
                        print!("{:02X} ", block.get::<u8>(offset));
                    } else {
                        print!("{:02X} ", pattern_block.get::<u8>(offset));
                    }
                }
                println!();
            }
            return;
        }
    }
    info!("Test Success");
}

fn test_fs() {
    let dev_name = "ram0";

    let root = open_dir(dev_name, 0, "/", b"r").expect("Attempt to Open Root Directory Failed");
    let mut count = 0;
    for (idx, entry) in root.entries() {
        info!("[{idx}] /{entry}");
        count += 1;
    }
    info!("Total {count} entries");

    let mut buffer = [0u8; 11];
    if let Ok(mut file) = open(dev_name, 0, "/file", b"r") {
        info!("File Found");
        file.read(&mut buffer).expect("File Read Failed");
        info!("File data={buffer:?}");
        file.remove().expect("File Remove Failed");
        flush();
    } else {
        info!("File Not Found");
        let mut file = open(dev_name, 0, "/file", b"w").expect("File Create Failed");
        buffer = [4u8; 11];
        info!("File data={buffer:?}");
        file.write(&buffer).expect("File Write Failed");
        info!("File Write Complete");
        flush();
    }

    let root = open_dir(dev_name, 0, "/", b"r").expect("Attempt to Open Root Directory Failed");
    let mut count = 0;
    for (idx, entry) in root.entries() {
        info!("[{idx}] /{entry}");
        count += 1;
    }
    info!("Total {count} entries");
}

fn test() {
    for i in 0..50 {
        create_task(
            TaskFlags::new().thread().set_priority(66).clone(),
            None,
            test_thread as u64,
            0,
            0,
        );
    }
    for i in 0..50 {
        create_task(
            TaskFlags::new().thread().set_priority(130).clone(),
            None,
            test_thread as u64,
            0,
            0,
        );
    }
    for i in 0..50 {
        create_task(
            TaskFlags::new().thread().set_priority(200).clone(),
            None,
            test_thread as u64,
            0,
            0,
        );
    }
    loop {
        schedule();
    }
}

fn test_fpu() {
    let id = running_task().unwrap().id() + 1;
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
    let id = running_task().unwrap().id() + 1;

    let mut random = id;
    let mut value1 = 1f64;
    let mut value2 = 1f64;

    let data = [b'-', b'\\', b'|', b'/'];
    let mut count = 0;

    let mut writer = create_window(8, 16);

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
            0,
            0,
            data[count],
            PixelColor::Red,
            PixelColor::Black,
            &mut writer,
        );
        // render();
        count = (count + 1) % 4;
    }
    write_ascii(0, 0, b' ', PixelColor::Red, PixelColor::White, &mut writer);
    info!("Thread id={id}: FPU Test Failed -> left={value1}, right={value2}");
}

fn test_windmill() {
    let id = running_task().unwrap().id() + 1;
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
            &mut get_graphic().lock(),
        );
        count = (count + 1) % 4;
    }
}
