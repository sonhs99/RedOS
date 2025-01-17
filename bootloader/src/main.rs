#![no_main]
#![no_std]

#[macro_use]
extern crate alloc;

use core::{
    mem::transmute,
    ptr::{copy_nonoverlapping, slice_from_raw_parts_mut, write_bytes},
};

use bootloader::{acpi::RSDP, BootInfo, FrameBufferConfig, MemoryMap};
use elflib::{Elf64, PT_LOAD};
use log::info;
use uefi::{
    data_types::PhysicalAddress,
    prelude::*,
    proto::{
        console::gop::{GraphicsOutput, PixelFormat},
        loaded_image::LoadedImage,
        media::{
            file::{Directory, File, FileAttribute, FileInfo, FileMode, RegularFile},
            fs::SimpleFileSystem,
        },
    },
    table::{
        boot::{self, AllocateType, BootServices, MemoryType},
        cfg::ACPI2_GUID,
    },
};

type EntryPoint = extern "sysv64" fn(BootInfo);

#[entry]
fn main(image_handle: Handle, mut system_table: SystemTable<Boot>) -> Status {
    uefi_services::init(&mut system_table).unwrap();

    let (kernel_entry_point, boot_info) = {
        info!("Boot Start!");

        let bs = system_table.boot_services();
        let mut root_dir = open_root_dir(bs);

        // Kernel Load
        let mut kernel_file = root_dir
            .open(
                cstr16!("\\kernel.elf"),
                FileMode::Read,
                FileAttribute::empty(),
            )
            .unwrap()
            .into_regular_file()
            .unwrap();
        let mut file_info_buffer = [0u8; 0x100];
        let file_info = kernel_file
            .get_info::<FileInfo>(&mut file_info_buffer)
            .unwrap();

        let kernel_base_addr = 0x100000 as PhysicalAddress;
        let kernel_file_size = file_info.file_size() as usize;

        let kernel_buffer = bs
            .allocate_pool(MemoryType::LOADER_DATA, kernel_file_size)
            .unwrap();
        kernel_file.read(unsafe {
            &mut *slice_from_raw_parts_mut(kernel_buffer as *mut u8, kernel_file_size)
        });

        let elf_file = Elf64::new(kernel_buffer as u64);
        let (kernel_first_addr, kernel_last_addr) = calculate_address(&elf_file);
        let num_page = (kernel_last_addr - kernel_first_addr + 0x0FFF) / 0x1000;
        bs.allocate_pages(
            AllocateType::Address(kernel_first_addr),
            MemoryType::LOADER_DATA,
            num_page as usize,
        )
        .unwrap();
        copy_load_segment(&elf_file);

        info!("[KERNEL] Range: 0x{kernel_first_addr:08X} - 0x{kernel_last_addr:08X}");
        let header = elf_file.get_header();
        info!("[KERNEL] Entry Point: 0x{:08X}", header.e_entry);
        info!("[KERNEL] Type: 0x{:04X}", header.e_type);
        info!("[KERNEL] Pages: {}", num_page);

        let kernel_entry_point = header.e_entry;

        unsafe { bs.free_pool(kernel_buffer).unwrap() };

        // Stack

        let stack_start_addr = (kernel_last_addr + 0xF_FFFF) & !0xF_FFFF;
        let stack_size = 0x10_0000;
        let stack_last_addr = stack_start_addr + stack_size;
        bs.allocate_pages(
            AllocateType::Address(stack_start_addr),
            MemoryType::LOADER_DATA,
            stack_size as usize / 0x1000,
        )
        .unwrap();

        info!("[STACK] Range: 0x{stack_start_addr:08X} - 0x{stack_last_addr:08X}");
        info!("[STACK] Pages: {}", stack_size / 0x1000);

        // IST

        let ist_start_addr = (stack_last_addr + 0xF_FFFF) & !0xF_FFFF;
        let ist_size = 0x10_0000;
        let ist_last_addr = ist_start_addr + ist_size;
        bs.allocate_pages(
            AllocateType::Address(ist_start_addr),
            MemoryType::LOADER_DATA,
            stack_size as usize / 0x1000,
        )
        .unwrap();
        info!("[IST  ] Range: 0x{ist_start_addr:08X} - 0x{ist_last_addr:08X}");
        info!("[IST  ] Pages: {}", ist_size / 0x1000);

        // Trampoline Load
        let ap_bootstrap = match root_dir.open(
            cstr16!("\\ap_bootstrap.bin"),
            FileMode::Read,
            FileAttribute::empty(),
        ) {
            Ok(mut ap_file) => {
                let mut ap_file = ap_file.into_regular_file().unwrap();
                let mut file_info_buffer = [0u8; 0x100];
                let file_info = ap_file.get_info::<FileInfo>(&mut file_info_buffer).unwrap();

                let ap_file_size = file_info.file_size() as usize;
                let num_page = (ap_file_size + 0xFFF) / 0x1000;

                // let ap_base_addr = 0x8000;
                let ap_base_addr = bs
                    .allocate_pages(
                        AllocateType::Address(0x8000),
                        MemoryType::LOADER_DATA,
                        num_page,
                    )
                    .unwrap();

                ap_file.read(unsafe {
                    &mut *slice_from_raw_parts_mut(ap_base_addr as *mut u8, kernel_file_size)
                });

                info!(
                    "[TRAMP] Range: 0x{:08X} - 0x{:08X}",
                    ap_base_addr,
                    ap_base_addr + ap_file_size as u64
                );
                info!("[TRAMP] Pages: {}", num_page);
                Some(ap_base_addr as u32)
            }
            Err(_) => {
                info!("Not Found AP Kernel");
                None
            }
        };

        // Memory Map Load

        let mmap_size = bs.memory_map_size();
        let mmap_byte = mmap_size.map_size + (mmap_size.entry_size * 5);
        let mmap_buf = bs
            .allocate_pool(MemoryType::RUNTIME_SERVICES_DATA, mmap_byte)
            .unwrap();
        let mmap_ref = unsafe { &mut *slice_from_raw_parts_mut(mmap_buf, mmap_byte) };
        let memory_map = bs.memory_map(mmap_ref).expect("Cannot Get Memory Map");
        let mut mmap_file = root_dir
            .open(
                cstr16!("\\memmap.txt"),
                FileMode::CreateReadWrite,
                FileAttribute::empty(),
            )
            .unwrap()
            .into_regular_file()
            .unwrap();
        save_memory_map(&mut mmap_file, &memory_map);

        // GOP
        let gop_handle = bs.get_handle_for_protocol::<GraphicsOutput>().unwrap();
        let mut gop = bs
            .open_protocol_exclusive::<GraphicsOutput>(gop_handle)
            .unwrap();
        let (height, width) = gop.current_mode_info().resolution();
        info!(
            "Resolution: {}x{}, Pixel Format: {:#?}, {} pixel/line",
            height,
            width,
            gop.current_mode_info().pixel_format(),
            gop.current_mode_info().stride()
        );
        info!(
            "Frame Buffer: 0x{:0X}, Size: {} bytes",
            gop.frame_buffer().as_mut_ptr() as u64,
            gop.frame_buffer().size(),
        );

        let pixel_format = match gop.current_mode_info().pixel_format() {
            PixelFormat::Rgb => bootloader::PixelFormat::RGBReserved8,
            PixelFormat::Bgr => bootloader::PixelFormat::BGRReserved8,
            PixelFormat::Bitmask => bootloader::PixelFormat::Bitmask,
            PixelFormat::BltOnly => bootloader::PixelFormat::BltOnly,
        };

        //ACPI
        let rsdp = unsafe {
            system_table
                .config_table()
                .iter()
                .find(|&entry| entry.guid == ACPI2_GUID)
                .expect("ACPI Not Found")
                .address
                .cast::<RSDP>()
                .as_ref()
                .unwrap()
        };

        (
            kernel_entry_point,
            BootInfo {
                frame_config: FrameBufferConfig::new(
                    height,
                    width,
                    gop.current_mode_info().stride(),
                    gop.frame_buffer().as_mut_ptr() as u64,
                    pixel_format,
                ),
                memory_map: MemoryMap {
                    buffer_size: mmap_byte as u64,
                    buffer: mmap_buf,
                    map_size: mmap_size.map_size as u64,
                    descriptor_size: mmap_size.entry_size as u64,
                },
                rsdp,
                ap_bootstrap,
                stack_frame: (stack_start_addr, stack_size as usize),
                ist_frame: (ist_start_addr, ist_size as usize),
            },
        )
    };

    system_table.exit_boot_services(MemoryType::LOADER_DATA);

    let kernel_entry_point: EntryPoint = unsafe { transmute(kernel_entry_point) };
    (kernel_entry_point)(boot_info);

    loop {}
    Status::SUCCESS
}

fn open_root_dir(bs: &BootServices) -> Directory {
    let loaded_image = bs
        .open_protocol_exclusive::<LoadedImage>(bs.image_handle())
        .unwrap();
    let mut fs = bs
        .open_protocol_exclusive::<SimpleFileSystem>(loaded_image.device().unwrap())
        .unwrap();
    fs.open_volume().unwrap()
}

fn save_memory_map(file: &mut RegularFile, memory_map: &boot::MemoryMap) {
    for (idx, entry) in memory_map.entries().enumerate() {
        let buf = format!(
            "{}, {:#?}, 0x{:08X}, {:X}, {:X}\n",
            idx,
            entry.ty,
            entry.phys_start,
            entry.page_count,
            entry.att.bits() & 0xFFFFF
        );
        let _ = file.write(buf.as_bytes());
    }
}

fn calculate_address(elf_file: &Elf64) -> (u64, u64) {
    let (mut start, mut end) = (u64::MAX, u64::MIN);
    for pheader in elf_file.get_pheader_iter() {
        if pheader.p_type != PT_LOAD {
            continue;
        }
        start = start.min(pheader.p_vaddr);
        end = end.max(pheader.p_vaddr + pheader.p_memsz);
    }
    (start, end)
}

fn copy_load_segment(elf_file: &Elf64) {
    for pheader in elf_file.get_pheader_iter() {
        if pheader.p_type != PT_LOAD {
            continue;
        }
        let seg_in_file = elf_file.start_address + pheader.p_offset;
        unsafe {
            copy_nonoverlapping(
                seg_in_file as *const u8,
                pheader.p_vaddr as *mut u8,
                pheader.p_filesz as usize,
            )
        };
        let remain_bytes = pheader.p_memsz - pheader.p_filesz;
        unsafe {
            write_bytes(
                (pheader.p_vaddr + pheader.p_filesz) as *mut u8,
                0,
                remain_bytes as usize,
            )
        };
    }
}
