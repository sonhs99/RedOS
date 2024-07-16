use super::{
    msi::{MsiCapabilityRegister, MsiXCapabilityRegister},
    Pci, PciDevice,
};

#[derive(Debug)]
pub enum CapabilityId {
    MSI = 0x05,
    MSIX = 0x11,
    Unknown,
}

pub struct CapabilityRegister<'dev> {
    device: &'dev PciDevice,
    offset: u8,
}

impl<'dev> CapabilityRegister<'dev> {
    pub fn id(&self) -> CapabilityId {
        let id = Pci::read_config(self.device, self.offset) as u8;
        match id {
            0x05 => CapabilityId::MSI,
            0x11 => CapabilityId::MSIX,
            _ => CapabilityId::Unknown,
        }
    }

    pub fn msi(&self) -> Option<MsiCapabilityRegister> {
        match self.id() {
            CapabilityId::MSI => Some(MsiCapabilityRegister {
                device: self.device,
                offset: self.offset,
            }),
            _ => None,
        }
    }

    pub fn msix(&self) -> Option<MsiXCapabilityRegister> {
        match self.id() {
            CapabilityId::MSIX => Some(MsiXCapabilityRegister {
                device: self.device,
                offset: self.offset,
            }),
            _ => None,
        }
    }
}

pub struct CapabilityRegisterIter<'dev> {
    pub device: &'dev PciDevice,
    pub offset: u8,
}

impl<'dev> Iterator for CapabilityRegisterIter<'dev> {
    type Item = CapabilityRegister<'dev>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.offset == 0 {
            return None;
        }
        let header = Pci::read_config(self.device, self.offset);
        let new_offset = (header >> 8) as u8;
        let old_offset = self.offset;
        self.offset = new_offset;
        Some(CapabilityRegister {
            device: self.device,
            offset: old_offset,
        })
    }
}
