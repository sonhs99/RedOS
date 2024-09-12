use core::ops::{Deref, DerefMut};
use core::ptr::{addr_of, addr_of_mut, read_unaligned, write_unaligned};

use alloc::boxed::Box;

use crate::sync::{Mutex, MutexGuard};

pub mod pata;
pub mod ram;

#[derive(Clone)]
pub struct Block<const N: usize>([u8; N]);

pub trait BlockIO {
    fn read(&mut self, address: u32, buffer: &mut [Block<512>]) -> Result<usize, usize>;
    fn write(&mut self, address: u32, buffer: &[Block<512>]) -> Result<usize, usize>;
    fn max_addr(&self) -> u32;
}

impl BlockIO for Mutex<Box<dyn BlockIO>> {
    fn read(&mut self, address: u32, buffer: &mut [Block<512>]) -> Result<usize, usize> {
        self.lock().read(address, buffer)
    }

    fn write(&mut self, address: u32, buffer: &[Block<512>]) -> Result<usize, usize> {
        self.lock().write(address, buffer)
    }

    fn max_addr(&self) -> u32 {
        self.lock().max_addr()
    }
}

impl<const N: usize> Block<N> {
    pub const fn empty() -> Self {
        Self([0u8; N])
    }
    pub fn get<T>(&self, index: usize) -> &T {
        let ptr = self.0.as_ptr().cast::<T>();
        unsafe { &*ptr.add(index) }
    }

    pub fn get_mut<T>(&mut self, index: usize) -> &mut T {
        let ptr = self.0.as_mut_ptr().cast::<T>();
        unsafe { &mut *ptr.add(index) }
    }
}

impl<const N: usize> Default for Block<N> {
    fn default() -> Self {
        Self([0u8; N])
    }
}

impl Block<512> {
    pub fn mbr(&self) -> &MBR {
        unsafe { &*(self as *const Self).cast::<MBR>() }
    }

    pub fn mbr_mut(&mut self) -> &mut MBR {
        unsafe { &mut *(self as *mut Self).cast::<MBR>() }
    }

    pub fn convert<T>(&self) -> &T {
        unsafe { &*(self as *const Self).cast::<T>() }
    }
}

#[derive(Clone, Copy, Debug)]
#[repr(C, packed)]
pub struct MBRPartionEntry {
    status: u8,
    start_chs_head: u8,
    start_chs_sector: u16,
    type_: u8,
    end_chs_head: u8,
    end_chs_sector: u16,
    start_lba: u32,
    sector_count: u32,
}

impl MBRPartionEntry {
    pub fn start_address(&self) -> u32 {
        unsafe { read_unaligned(addr_of!(self.start_lba)) }
    }

    pub fn size(&self) -> u32 {
        unsafe { read_unaligned(addr_of!(self.sector_count)) }
    }

    pub fn type_(&self) -> u8 {
        self.type_
    }

    pub fn set_bootable(&mut self) {
        self.status = 0x80;
    }

    pub fn set_start_address(&mut self, start_addr: u32) -> &mut Self {
        unsafe { write_unaligned(addr_of_mut!(self.start_lba), start_addr) };
        self
    }

    pub fn set_size(&mut self, size: u32) -> &mut Self {
        unsafe { write_unaligned(addr_of_mut!(self.sector_count), size) }
        self
    }

    pub fn set_type_(&mut self, type_: u8) -> &mut Self {
        self.type_ = type_;
        self
    }
}

#[derive(Clone, Copy)]
#[repr(C, packed)]
pub struct MBR {
    boot_code: [u8; 446],
    partition: [MBRPartionEntry; 4],
    magic_word: [u8; 2],
}

impl MBR {
    pub fn partition(&self, index: usize) -> MBRPartionEntry {
        unsafe { read_unaligned(addr_of!(self.partition))[index] }
    }

    pub fn set_partition(&mut self, index: usize, partition: MBRPartionEntry) {
        unsafe { write_unaligned(addr_of_mut!(self.partition[index]), partition) };
    }
}
