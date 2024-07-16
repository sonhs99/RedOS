use log::debug;

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

impl<'dev> Msi for MsiXCapabilityRegister<'dev> {
    fn enable(&self, message: &Message) {
        let header = Pci::read_config(self.device, self.offset);
        let table_size = (header >> 16) & 0x07FF + 1;
        let header = header | 0x8000_0000u32;
        Pci::write_config(self.device, self.offset, header);

        let table_st = Pci::read_config(self.device, self.offset + 4);
        let bar = table_st & 0x07;
        let bar = self.device.read_bar(bar as u8);
        let bar = bar & !0x0F;
        let offset = (table_st & !0x07) as u64;
        let table_ptr = (bar + offset) as *mut u32;

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
