use alloc::vec::Vec;

use super::{PciDevice, PCI_BUS};

pub enum Base {
    Serial,
    MassStorage,
}

impl Base {
    #[inline(always)]
    pub const fn as_u8(self) -> Option<u8> {
        match self {
            Self::Serial => Some(0x0C),
            Self::MassStorage => Some(0x01),
            _ => None,
        }
    }
}

pub enum Sub {
    USB,
    IDE,
}

impl Sub {
    #[inline(always)]
    pub const fn as_u8(self) -> Option<u8> {
        match self {
            Self::USB => Some(0x03),
            Self::IDE => Some(0x01),
            _ => None,
        }
    }
}

pub enum Interface {
    UHC,
    OHCI,
    EHCI,
    XHCI,
    None,
}

impl Interface {
    #[inline(always)]
    pub const fn as_u8(self) -> Option<u8> {
        match self {
            Self::OHCI => Some(0x10),
            Self::EHCI => Some(0x20),
            Self::XHCI => Some(0x30),
            Self::None => Some(0xFF),
            _ => None,
        }
    }
}

pub struct PciSearcher {
    base: u8,
    sub: u8,
    interface: u8,
}

impl PciSearcher {
    pub fn new() -> Self {
        Self {
            base: 0xFF,
            sub: 0xFF,
            interface: 0xFF,
        }
    }

    pub fn base(mut self, base: Base) -> Self {
        self.base = base.as_u8().unwrap();
        self
    }

    pub fn sub(mut self, sub: Sub) -> Self {
        self.sub = sub.as_u8().unwrap();
        self
    }

    pub fn interface(mut self, interface: Interface) -> Self {
        self.interface = interface.as_u8().unwrap();
        self
    }

    pub fn search(&self) -> Option<Vec<PciDevice>> {
        if self.base == 0xFF {
            return None;
        }
        let devices: Vec<_> = PCI_BUS
            .lock()
            .device_iter()
            .filter(|device| {
                device
                    .class_code
                    .is_class(self.base, self.sub, self.interface)
            })
            .collect();
        if devices.is_empty() {
            None
        } else {
            Some(devices)
        }
    }
}
