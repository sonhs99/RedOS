use core::{
    ptr::{read_volatile, write_volatile},
    u64,
};

use alloc::vec::Vec;
use log::debug;

use crate::{interrupt::apic::LocalAPICRegisters, percpu::get_cpu_count, sync::OnceLock};

use super::{Pci, PciDevice};

pub struct Message {
    dest_id: u8,
    dest_mode: bool,
    redirection: bool,

    int_vec: u8,
    trigger: bool,
    level: bool,
    delivery_mode: u8,
}

impl Message {
    pub fn new() -> Self {
        Self {
            dest_id: 0,
            dest_mode: false,
            redirection: false,
            int_vec: 0,
            trigger: false,
            level: false,
            delivery_mode: 0,
        }
    }

    pub fn destionation_id(mut self, id: u8) -> Self {
        self.dest_id = id;
        self
    }

    pub fn destionation_mode(mut self, mode: bool) -> Self {
        self.dest_mode = mode;
        self
    }

    pub fn redirection(mut self, hint: bool) -> Self {
        self.redirection = hint;
        self
    }

    pub fn interrupt_index(mut self, index: u8) -> Self {
        self.int_vec = index;
        self
    }

    pub fn trigger_mode(mut self, trigger: bool) -> Self {
        self.trigger = trigger;
        self
    }

    pub fn level(mut self, level: bool) -> Self {
        self.level = level;
        self
    }

    pub fn delivery_mode(mut self, mode: u8) -> Self {
        self.delivery_mode = mode & 0x07;
        self
    }

    pub fn address(&self) -> u32 {
        let mut address = 0xFEE0_0000 as u32;
        address |= (self.dest_id as u32) << 12;
        address |= (self.redirection as u32) << 3;
        address |= (self.dest_mode as u32) << 2;
        address
    }

    pub fn data(&self) -> u32 {
        let mut data = self.int_vec as u32;
        data |= (self.delivery_mode as u32) << 8;
        data |= (self.level as u32) << 14;
        data |= (self.trigger as u32) << 15;
        data
    }
}

pub trait Msi {
    fn enable(&self, message: &Message);
}

pub struct MsiCapabilityRegister<'dev> {
    pub device: &'dev PciDevice,
    pub offset: u8,
}

impl<'dev> Msi for MsiCapabilityRegister<'dev> {
    fn enable(&self, message: &Message) {
        let header = Pci::read_config(self.device, self.offset);
        let muiti_msg = (header >> 17) & 0x07;
        let header = header | (muiti_msg << 20) | 0x010000u32;
        Pci::write_config(self.device, self.offset, header);

        let address = message.address();
        Pci::write_config(self.device, self.offset + 4, address);

        let data = message.data();
        Pci::write_config(self.device, self.offset + 12, data);
    }
}

pub struct MsiXCapabilityRegister<'dev> {
    pub device: &'dev PciDevice,
    pub offset: u8,
}

impl<'dev> MsiXCapabilityRegister<'dev> {
    pub fn table(&self) -> u64 {
        let table_st = Pci::read_config(self.device, self.offset + 4);
        let bar = table_st & 0x07;
        let bar = self.device.read_bar(bar as u8);
        let bar = bar & !0x0F;
        let offset = (table_st & !0x07) as u64;
        bar + offset
    }
}

impl<'dev> Msi for MsiXCapabilityRegister<'dev> {
    fn enable(&self, message: &Message) {
        let header = Pci::read_config(self.device, self.offset);
        let table_size = (header >> 16) & 0x07FF + 1;
        let header = header | 0x8000_0000u32;
        Pci::write_config(self.device, self.offset, header);

        let table_ptr = self.table() as *mut u32;

        let address = message.address();
        let data = message.data();

        debug!(
            "table_ptr={:#018X}, addr={:#010X}, data={:#010X}",
            table_ptr as u64, address, data
        );

        for i in 0..=table_size as usize {
            unsafe {
                *table_ptr.add(i * 4) = address;
                *table_ptr.add(i * 4 + 1) = 0;
                *table_ptr.add(i * 4 + 2) = data;
                *table_ptr.add(i * 4 + 3) = 0;
            }
        }
    }
}

pub enum MSIEntry {
    MSI(PciDevice, u8),
    MSIX(u64, u8),
}

static mut IRQ_INTERRUPT_COUNT: [[u64; 2]; 16] = [[0; 2]; 16];
static INT_ROUTING_TABLE: OnceLock<Vec<MSIEntry>> = OnceLock::new();

const LOADBALANCING_DIVIDER: u64 = 10;

pub fn init_routing_table(table: Vec<MSIEntry>) {
    INT_ROUTING_TABLE.get_or_init(|| table);
}

pub fn increase_int_count(irq: u8) {
    let apic_id = LocalAPICRegisters::default().local_apic_id().id();
    unsafe { IRQ_INTERRUPT_COUNT[apic_id as usize][irq as usize] += 1 };
}

pub fn load_balance_int(irq: u8) {
    let apic_id = LocalAPICRegisters::default().local_apic_id().id();
    unsafe {
        if IRQ_INTERRUPT_COUNT[apic_id as usize][irq as usize] % LOADBALANCING_DIVIDER != 0
            || IRQ_INTERRUPT_COUNT[apic_id as usize][irq as usize] == 0
        {
            return;
        }
        let mut min_counted_core = 0u8;
        let mut min_count = u64::MAX;
        let mut reset_flag = false;
        for core in 0..get_cpu_count() as u8 {
            if IRQ_INTERRUPT_COUNT[core as usize][irq as usize] < min_count {
                min_count = IRQ_INTERRUPT_COUNT[core as usize][irq as usize];
                min_counted_core = core;
            } else if IRQ_INTERRUPT_COUNT[core as usize][irq as usize] == u64::MAX {
                reset_flag = true;
            }
        }
        // debug!("Routing {min_counted_core}");

        routing_int(irq, min_counted_core);

        if reset_flag {
            for core in 0..get_cpu_count() {
                IRQ_INTERRUPT_COUNT[core][irq as usize] == 0;
            }
        }
    }
}

fn routing_int(irq: u8, id: u8) {
    if let Some(table) = INT_ROUTING_TABLE.get() {
        match table[irq as usize] {
            MSIEntry::MSI(device, offset) => {
                Pci::write_config(&device, offset + 4, 0xFEE0_0000 | (id as u32) << 12);
            }
            MSIEntry::MSIX(table_ptr, table_offset) => unsafe {
                let ptr = table_ptr + table_offset as u64 * 128;
                let address = read_volatile(ptr as *const u32);
                let address = (address & !0x000F_F000) | (id as u32) << 12;
                write_volatile(ptr as *mut u32, address);
            },
        }
    }
}
