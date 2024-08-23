use core::ptr::{addr_of, read_unaligned};
use core::{ptr::slice_from_raw_parts, slice};

use alloc::vec;
use alloc::{boxed::Box, vec::Vec};
use log::debug;

use super::{DirectoryDescriptor, FileDescriptor, FileSystem};
use crate::device::block::{Block, BlockIO};

// in progress
// pub mod fat16;
pub mod fat32;

pub use fat32::FAT32;

pub const FAT_SECTOR_PER_CLUSTER: u8 = 8;
pub const FAT_SECTOR_PER_CLUSTER_ENTRY: u32 = 512 / 4;
pub const FAT_MAX_DIRECTORY_ENTRY_COUNT: u32 = 128;

pub const FAT_END_OF_CLUSTER: u32 = 0x0FFF_FFFF;

pub const FAT_DIR_ATTRIBUTE_NIL: u8 = 0x00;
pub const FAT_DIR_ATTRIBUTE_FILE: u8 = 0x01;
pub const FAT_DIR_ATTRIBUTE_DIR: u8 = 0x02;

pub enum FATType {
    FAT16,
    FAT32,
}

#[repr(C, packed)]
struct CommonFATHeader {
    jmp_boot_code: [u8; 3],
    oem_id: [u8; 8],
    byte_per_sector: u16,
    sector_per_clustor: u8,
    reserved_sector_count: u16,
    num_fat_table: u8,
    root_dir_entry_count: u16,
    total_sector16: u16,
    media_type: u8,
    fat_size16: u16,
    sector_per_track: u16,
    num_head: u16,
    hidden_sector: u32,
    total_sector32: u32,
}

impl CommonFATHeader {
    pub const fn empty() -> Self {
        Self {
            jmp_boot_code: [0; 3],
            oem_id: [0; 8],
            byte_per_sector: 0,
            sector_per_clustor: 0,
            reserved_sector_count: 0,
            num_fat_table: 0,
            root_dir_entry_count: 0,
            total_sector16: 0,
            media_type: 0,
            fat_size16: 0,
            sector_per_track: 0,
            num_head: 0,
            hidden_sector: 0,
            total_sector32: 0,
        }
    }
}

#[derive(Clone, Copy)]
#[repr(C)]
struct DirectoryEntry {
    name: [u8; 11],
    attr: u8,
    _reserved: u8,
    _time1: [u8; 7],
    cluster_high: u16,
    _time2: [u8; 4],
    cluster_low: u16,
    file_size: u32,
}

impl DirectoryEntry {
    pub const fn empty() -> Self {
        Self {
            name: [0x20; 11],
            attr: 0,
            _reserved: 0,
            _time1: [0; 7],
            cluster_high: 0,
            _time2: [0; 4],
            cluster_low: 0,
            file_size: 0,
        }
    }
    pub fn start_cluster_idx(&self) -> u32 {
        self.cluster_low as u32 | (self.cluster_high as u32) << 16
    }

    pub fn set_start_cluster_idx(&mut self, cluster_addr: u32) {
        self.cluster_low = cluster_addr as u16;
        self.cluster_high = (cluster_addr >> 16) as u16;
    }
}

pub fn fat_type(vbr: &Block<512>) -> FATType {
    let header = vbr.convert::<CommonFATHeader>();
    unsafe { debug!("size16={}", read_unaligned(addr_of!(header.fat_size16))) };
    if header.fat_size16 != 0 {
        FATType::FAT16
    } else {
        FATType::FAT32
    }
}
