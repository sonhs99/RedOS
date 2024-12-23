use core::{
    intrinsics::compare_bytes,
    ptr::{addr_of, read_unaligned, slice_from_raw_parts},
    slice::Iter,
};

use bootloader::acpi::RSDP;
use log::debug;

use crate::{
    interrupt::{apic::IOAPICRegister, InterruptVector},
    sync::OnceLock,
};

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

impl FADT {
    pub const fn timer(&self) -> u32 {
        self.pm_tmr_blk
    }

    pub const fn flags(&self) -> u32 {
        self.flags
    }
}

#[repr(C, packed)]
pub struct MADT {
    header: DescriptionHeader,
    lapic_addr: u32,
    flags: u32,
}

#[repr(C, packed)]
pub struct MADTHeader {
    pub type_: u8,
    pub length: u8,
}

impl MADT {
    pub fn entries(&self) -> MADTIter {
        let base = (self as *const Self).cast::<u8>();
        let base = unsafe { base.add(size_of::<Self>()) };
        MADTIter {
            base,
            idx: 0,
            length: self.header.length,
        }
    }
}
pub struct MADTIter {
    base: *const u8,
    idx: u32,
    length: u32,
}

impl Iterator for MADTIter {
    type Item = &'static MADTHeader;

    fn next(&mut self) -> Option<Self::Item> {
        let header = unsafe { &*self.base.add(self.idx as usize).cast::<MADTHeader>() };
        let endpoint = self.idx + header.length as u32;
        if endpoint >= self.length || endpoint == self.idx {
            return None;
        }
        self.idx = endpoint;
        Some(header)
    }
}

#[repr(C, packed)]
pub struct LocalAPICEntry {
    pub header: MADTHeader,
    pub uid: u8,
    pub id: u8,
    pub flags: u32,
}

#[repr(C, packed)]
pub struct IOAPICEntry {
    pub header: MADTHeader,
    pub id: u8,
    _reserved: u8,
    pub address: u32,
    pub global_addr: u32,
}

#[repr(C, packed)]
pub struct IntOverrideEntry {
    pub header: MADTHeader,
    pub bus: u8,
    pub source: u8,
    pub global_int: u32,
    pub flags: u16,
}

#[repr(C, packed)]
pub struct IntNMIOverrideEntry {
    pub header: MADTHeader,
    pub flags: u16,
    pub global_int: u32,
}

#[repr(C, packed)]
pub struct LocalOverrideEntry {
    pub header: MADTHeader,
    _reserved: u16,
    pub address: u64,
}

#[repr(C, packed)]
pub struct LocalNMIOverideEntry {
    pub header: MADTHeader,
    pub uid: u8,
    pub flags: u16,
    pub int: u8,
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
pub static MADT_CELL: OnceLock<&MADT> = OnceLock::new();

pub fn initialize(rsdp: &RSDP) {
    if !rsdp.is_valid() {
        debug!("RSDP is not valid");
    }
    let xsdt = unsafe { (rsdp.xsdt_address as *const XSDT).as_ref().unwrap() };
    if !xsdt.header.is_valid(b"XSDT") {
        debug!("XSDT is not valid");
    }

    if let Some(fadt) = xsdt.entries().find(|&entry| entry.is_valid(b"FACP")) {
        FADT_CELL.get_or_init(|| unsafe { &*(fadt as *const DescriptionHeader).cast::<FADT>() });
    } else {
        debug!("FADT is not found");
    }
    if let Some(madt) = xsdt.entries().find(|entry| entry.is_valid(b"APIC")) {
        MADT_CELL.get_or_init(|| unsafe { &*(madt as *const DescriptionHeader).cast::<MADT>() });
        debug!("MADT Length={}", unsafe {
            read_unaligned(addr_of!(madt.length))
        });
    } else {
        debug!("MADT is not found");
    }
}
