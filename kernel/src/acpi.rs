use core::{
    intrinsics::compare_bytes,
    ptr::{read_unaligned, slice_from_raw_parts},
    slice::Iter,
};

use bootloader::acpi::RSDP;
use log::debug;

use crate::sync::OnceLock;

#[repr(C, packed)]
pub struct DescriptionHeader {
    signature: [u8; 4],
    length: u32,
    revision: u8,
    checksum: u8,
    oem_id: [u8; 6],
    oem_table_id: [u8; 8],
    oem_revision: u32,
    creator_id: u32,
    creator_revision: u32,
}

struct XSDT {
    header: DescriptionHeader,
}

pub struct XSDTIter {
    ptr: *const u64,
    size: usize,
}

impl DescriptionHeader {
    pub fn is_valid(&self, expect_signature: &[u8]) -> bool {
        if unsafe { compare_bytes(self.signature.as_ptr(), expect_signature.as_ptr(), 4) != 0 } {
            false
        } else if sum(self as *const Self, self.length as usize) != 0 {
            false
        } else {
            true
        }
    }
}

impl XSDT {
    pub fn entries(&self) -> impl Iterator<Item = &'static DescriptionHeader> {
        let size =
            (self.header.length as usize - size_of::<DescriptionHeader>()) / size_of::<u64>();
        let base = &self.header as *const DescriptionHeader;
        unsafe {
            let entry_base = base.add(1).cast::<u64>();
            XSDTIter {
                ptr: entry_base,
                size,
            }
        }
    }
}

impl Iterator for XSDTIter {
    type Item = &'static DescriptionHeader;

    fn next(&mut self) -> Option<Self::Item> {
        if self.size == 0 {
            return None;
        }
        let entry = unsafe {
            let entry = read_unaligned(self.ptr) as *const DescriptionHeader;
            self.ptr = self.ptr.add(1);
            entry.as_ref().unwrap()
        };
        self.size -= 1;
        Some(entry)
    }
}

#[repr(C, packed)]
pub struct FADT {
    header: DescriptionHeader,
    _reserved1: [u8; 76 - size_of::<DescriptionHeader>()],
    pm_tmr_blk: u32,
    _reserved2: [u8; 112 - 80],
    flags: u32,
    _reserved3: [u8; 276 - 116],
}

fn sum<T>(ptr: *const T, size: usize) -> u8 {
    sum_inner(ptr.cast::<u8>(), size)
}

fn sum_inner(ptr: *const u8, size: usize) -> u8 {
    let mut count = 0u8;
    for idx in 0..size {
        unsafe {
            count = count.wrapping_add(*ptr.add(idx));
        }
    }
    count
}

pub static FADT_CELL: OnceLock<&FADT> = OnceLock::new();

pub fn initialize(rsdp: &RSDP) {
    if !rsdp.is_valid() {
        debug!("RSDP is not valid");
    }
    let xsdt = unsafe { (rsdp.xsdt_address as *const XSDT).as_ref().unwrap() };
    if !xsdt.header.is_valid(b"XSDT") {
        debug!("XSDT is not valid");
    }

    if let Some(fadt) = xsdt.entries().find(|&entry| entry.is_valid(b"FACP")) {
        FADT_CELL.get_or_init(|| unsafe {
            (fadt as *const DescriptionHeader)
                .cast::<FADT>()
                .as_ref()
                .unwrap()
        });
    } else {
        debug!("FADT is not found");
    }
}
