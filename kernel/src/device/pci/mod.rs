pub mod capability;
pub mod msi;
pub mod search;

use super::Port;
use crate::sync::{Mutex, OnceLock};
use capability::{CapabilityRegister, CapabilityRegisterIter};
use log::{debug, info};
use search::{Base, Interface, PciSearcher, Sub};

pub struct Pci {
    devices: [PciDevice; 32],
    num_device: u64,
}

#[derive(Clone, Copy)]
pub struct PciDevice {
    pub bus: u8,
    pub dev: u8,
    pub func: u8,
    pub header_type: u8,
    pub class_code: PciClass,
}

#[derive(Clone, Copy)]
pub struct PciClass {
    pub base: u8,
    pub sub: u8,
    pub interface: u8,
    pub revision_id: u8,
}

impl Pci {
    const CONFIG_ADDRESS: Port = Port::new(0x0cf8);
    const CONFIG_DATA: Port = Port::new(0x0cfc);

    pub fn new() -> Self {
        Self {
            devices: [PciDevice::new(0, 0, 0, 0, 0.into()); 32],
            num_device: 0,
        }
    }

    pub(self) fn write_address(addr: u32) {
        Self::CONFIG_ADDRESS.out32(addr);
    }

    pub(self) fn write_data(data: u32) {
        Self::CONFIG_DATA.out32(data);
    }

    pub(self) fn read_data() -> u32 {
        Self::CONFIG_DATA.in32()
    }

    pub fn read_config(dev: &PciDevice, addr: u8) -> u32 {
        Self::write_address(make_address(dev.bus, dev.dev, dev.func, addr));
        Self::read_data()
    }

    pub fn write_config(dev: &PciDevice, addr: u8, data: u32) {
        Self::write_address(make_address(dev.bus, dev.dev, dev.func, addr));
        Self::write_data(data);
    }

    pub fn read_vendor_id(bus: u8, device: u8, function: u8) -> u16 {
        Self::write_address(make_address(bus, device, function, 0x00));
        Self::read_data() as u16
    }

    pub fn read_header_type(bus: u8, device: u8, function: u8) -> u8 {
        Self::write_address(make_address(bus, device, function, 0x0C));
        (Self::read_data() >> 16) as u8
    }

    pub fn read_class_code(bus: u8, device: u8, function: u8) -> PciClass {
        Self::write_address(make_address(bus, device, function, 0x08));
        Self::read_data().into()
    }

    pub fn read_bus_numbers(bus: u8, device: u8, function: u8) -> u32 {
        Self::write_address(make_address(bus, device, function, 0x18));
        Self::read_data()
    }

    pub fn init(&mut self) -> Result<(), ()> {
        self.num_device = 0;
        let header_type = Self::read_header_type(0, 0, 0);
        if header_type & 0x80 == 0 {
            return self.scan_bus(0);
        }

        for func in 1..8 {
            if Self::read_vendor_id(0, 0, func) == 0xffff {
                continue;
            }
            self.scan_bus(func)?
        }
        Ok(())
    }

    pub fn scan_bus(&mut self, bus: u8) -> Result<(), ()> {
        for dev in 0..32 {
            if Self::read_vendor_id(bus, dev, 0) == 0xffff {
                continue;
            }
            self.scan_device(bus, dev)?
        }
        Ok(())
    }

    pub fn scan_device(&mut self, bus: u8, device: u8) -> Result<(), ()> {
        self.scan_function(bus, device, 0)?;

        let header_type = Self::read_header_type(bus, device, 0);
        if header_type & 0x80 == 0 {
            return Ok(());
        }
        for func in 1..8 {
            if Self::read_vendor_id(bus, device, func) == 0xffff {
                debug!("{bus}.{device}.{func}");
                continue;
            }
            self.scan_function(bus, device, func)?
        }
        Ok(())
    }

    pub fn scan_function(&mut self, bus: u8, device: u8, function: u8) -> Result<(), ()> {
        let header_type = Self::read_header_type(bus, device, function);
        let class_code = Self::read_class_code(bus, device, function);

        self.add_device(PciDevice::new(
            bus,
            device,
            function,
            header_type,
            class_code,
        ))?;

        if class_code.base == 0x06 && class_code.sub == 0x04 {
            let bus_numbers = Self::read_bus_numbers(bus, device, function);
            let secondary_bus = (bus_numbers >> 8) as u8;
            return self.scan_bus(secondary_bus);
        }
        Ok(())
    }

    pub fn add_device(&mut self, device: PciDevice) -> Result<(), ()> {
        if self.num_device == self.devices.len() as u64 {
            return Err(());
        }

        self.devices[self.num_device as usize] = device;
        self.num_device += 1;
        Ok(())
    }

    pub fn device_iter(&self) -> PciDeviceIterator {
        PciDeviceIterator {
            device: &self.devices[..(self.num_device as usize)],
            index: 0,
        }
        // &self.devices[0..(self.num_device as usize)]
    }
}

impl PciDevice {
    pub const fn new(bus: u8, dev: u8, func: u8, header_type: u8, class_code: PciClass) -> Self {
        Self {
            bus,
            dev,
            func,
            header_type,
            class_code,
        }
    }

    pub fn read_vendor_id(&self) -> u16 {
        Pci::read_vendor_id(self.bus, self.dev, self.func)
    }

    pub fn read_bar(&self, offset: u8) -> u64 {
        let bar_addr = 0x10 + offset * 4;
        Pci::write_address(make_address(self.bus, self.dev, self.func, bar_addr));
        let bar = Pci::read_data() as u64;
        if bar & 0x04 == 0 {
            bar
        } else {
            Pci::write_address(make_address(self.bus, self.dev, self.func, bar_addr + 4));
            let upper_bar = Pci::read_data() as u64;
            bar | upper_bar << 32
        }
    }

    pub fn capabilities(&self) -> CapabilityRegisterIter {
        let offset = Pci::read_config(self, 0x34) as u8;
        debug!("Capability offset={offset:#02X}");
        CapabilityRegisterIter {
            device: self,
            offset,
        }
    }
}

impl PciClass {
    pub fn is_class(&self, base: u8, sub: u8, interface: u8) -> bool {
        self.base == base && self.sub == sub && (self.interface == interface || interface == 0xFF)
    }
}

impl From<u32> for PciClass {
    fn from(value: u32) -> Self {
        Self {
            base: (value >> 24) as u8,
            sub: (value >> 16) as u8,
            interface: (value >> 8) as u8,
            revision_id: value as u8,
        }
    }
}

fn make_address(bus: u8, dev: u8, func: u8, addr: u8) -> u32 {
    1 << 31 | (bus as u32) << 16 | (dev as u32) << 11 | (func as u32) << 8 | (addr & 0xfc) as u32
}

pub struct PciDeviceIterator<'device> {
    device: &'device [PciDevice],
    index: usize,
}

impl<'device> Iterator for PciDeviceIterator<'device> {
    type Item = PciDevice;

    fn next(&mut self) -> Option<Self::Item> {
        let result = self.device.get(self.index)?;
        self.index += 1;
        Some(*result)
    }
}

static PCI_BUS: OnceLock<Mutex<Pci>> = OnceLock::new();

pub fn init_pci() {
    let pci = PCI_BUS.get_or_init(|| {
        let pci = Mutex::new(Pci::new());
        pci.lock().init().unwrap();
        pci
    });

    for dev in pci.lock().device_iter() {
        let vendor_id = dev.read_vendor_id();
        let class_code = dev.class_code;
        info!(
            "Address {}.{}.{}: vend {:04X}, class {:02X}{:02X}{:02X}{:02X}, head {:02x}",
            dev.bus,
            dev.dev,
            dev.func,
            vendor_id,
            class_code.base,
            class_code.sub,
            class_code.interface,
            class_code.revision_id,
            dev.header_type
        );
    }
}

pub fn switch_ehci_to_xhci(xhc_dev: &PciDevice) {
    let intel_ehc_exist = PciSearcher::new()
        .base(Base::Serial)
        .sub(Sub::USB)
        .interface(Interface::EHCI)
        .search();
    if let Some(_) = intel_ehc_exist {
        let superspeed_port = Pci::read_config(&xhc_dev, 0xdc);
        Pci::write_config(&xhc_dev, 0xd8, superspeed_port);
        let ehci2xhci_ports = Pci::read_config(&xhc_dev, 0xd4);
        Pci::write_config(&xhc_dev, 0xd0, ehci2xhci_ports);
        debug!("switch_ehci_to_xhci: SS = {superspeed_port:02X}, xHCI = {ehci2xhci_ports:02X}")
    }
}
