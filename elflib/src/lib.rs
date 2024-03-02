#![no_std]

use core::ptr::slice_from_raw_parts;

const EI_NDENT: usize = 16;

pub const PT_LOAD: u32 = 0x01;

#[repr(C)]
pub struct Elf64Header {
    pub e_ident: [u8; EI_NDENT],
    pub e_type: u16,
    pub e_machine: u16,
    pub e_version: u32,
    pub e_entry: u64,
    e_phoff: u64,
    e_shoff: u64,
    e_flags: u32,
    e_ehsize: u16,
    e_phentsize: u16,
    e_phnum: u16,
    e_shentsize: u16,
    e_shnum: u16,
    e_shstrndx: u16,
}

#[repr(C)]
pub struct Elf64PHeader {
    pub p_type: u32,
    pub p_flags: u32,
    pub p_offset: u64,
    pub p_vaddr: u64,
    pub p_paddr: u64,
    pub p_filesz: u64,
    pub p_memsz: u64,
    pub p_align: u64,
}

pub struct Elf64 {
    pub start_address: u64,
}

impl Elf64 {
    pub const fn new(start_address: u64) -> Self {
        Self { start_address }
    }

    pub fn get_header(&self) -> &'static Elf64Header {
        unsafe { &*(self.start_address as *const Elf64Header) }
    }

    pub fn get_pheader_iter(&self) -> &'static [Elf64PHeader] {
        let header = self.get_header();
        let pheader_start = self.start_address + header.e_phoff;
        unsafe {
            &*slice_from_raw_parts(
                pheader_start as *const Elf64PHeader,
                header.e_phnum as usize,
            )
        }
    }
}
