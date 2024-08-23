use core::ptr::{addr_of, read_unaligned};

use log::debug;

use crate::{
    acpi::{
        IOAPICEntry, IntNMIOverrideEntry, IntOverrideEntry, LocalAPICEntry, LocalNMIOverideEntry,
        LocalOverrideEntry, MADTHeader, MADT_CELL,
    },
    interrupt::{apic::IOAPICRegister, InterruptVector},
    sync::OnceLock,
};

struct IOAPICRedirectionTable {
    src: u8,
    flags: u8,
}

impl IOAPICRedirectionTable {
    pub const fn new(flags: u8) -> Self {
        Self { src: 0, flags }
    }
}

pub fn init() -> usize {
    let madt = *MADT_CELL.get().unwrap();
    let ioapic: OnceLock<IOAPICRegister> = OnceLock::new();
    let mut num_core = 0usize;
    let mut redirection_table = [const { IOAPICRedirectionTable::new(0x00) }; 16];
    for (idx, entry) in redirection_table.iter_mut().enumerate() {
        entry.src = idx as u8;
    }

    for (idx, entry) in madt.entries().enumerate() {
        match entry.type_ {
            0 => {
                let entry = unsafe { &*(entry as *const MADTHeader).cast::<LocalAPICEntry>() };
                num_core += 1;
                debug!(
                    "MADT Entry {}: Local APIC, id={} flags={:#X}",
                    idx,
                    entry.id,
                    unsafe { read_unaligned(addr_of!(entry.flags)) }
                );
            }
            1 => {
                let entry = unsafe { &*(entry as *const MADTHeader).cast::<IOAPICEntry>() };
                ioapic.get_or_init(|| IOAPICRegister::new(entry.address));
                debug!(
                    "MADT Entry {}: I/O APIC, id={} addr={:#X} global_addr={}",
                    idx,
                    entry.id,
                    unsafe { read_unaligned(addr_of!(entry.address)) },
                    unsafe { read_unaligned(addr_of!(entry.global_addr)) }
                );
            }
            2 => {
                let entry = unsafe { &*(entry as *const MADTHeader).cast::<IntOverrideEntry>() };
                let int = unsafe { read_unaligned(addr_of!(entry.global_int)) } as u8;
                let flags = unsafe { read_unaligned(addr_of!(entry.flags)) };

                debug!(
                    "MADT Entry {}: Interrupt Override, bus={} src={} glo={} flags={:#X}",
                    idx, entry.bus, entry.source, int, flags
                );
                redirection_table[int as usize].src = entry.source;
                redirection_table[int as usize].flags = flags as u8;
            }
            3 => {
                let entry = unsafe { &*(entry as *const MADTHeader).cast::<IntNMIOverrideEntry>() };
                debug!(
                    "MADT Entry {}: NMI Override, int={} flags={:#X}",
                    idx,
                    unsafe { read_unaligned(addr_of!(entry.global_int)) },
                    unsafe { read_unaligned(addr_of!(entry.flags)) }
                );
            }
            4 => {
                let entry =
                    unsafe { &*(entry as *const MADTHeader).cast::<LocalNMIOverideEntry>() };
                debug!(
                    "MADT Entry {}: Local NMI Override, int={} flags={:#X}",
                    idx,
                    entry.int,
                    unsafe { read_unaligned(addr_of!(entry.flags)) }
                );
            }
            5 => {
                let entry = unsafe { &*(entry as *const MADTHeader).cast::<LocalOverrideEntry>() };
                debug!(
                    "MADT Entry {}: Local Int Override, addr={:#X}",
                    idx,
                    unsafe { read_unaligned(addr_of!(entry.address)) },
                );
            }
            _ => {}
        }
    }

    let io_apic = ioapic.get_or_init(|| IOAPICRegister::default());
    for (int, entry) in redirection_table.iter().enumerate() {
        if entry.src == 0 {
            continue;
        }
        let high = 0x0000_0000u32;
        let mut low = (entry.src + InterruptVector::IRQStart as u8) as u32;
        if entry.flags & 0b0010 != 0 {
            low |= 0x2000;
        }
        if entry.flags & 0b1000 != 0 {
            low |= 0x8000;
        }
        io_apic.write(16 + int as u8 * 2, low);
        io_apic.write(16 + int as u8 * 2 + 1, high);
    }
    num_core
}
