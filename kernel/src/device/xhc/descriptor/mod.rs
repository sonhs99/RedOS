use core::mem::size_of;

use alloc::{boxed::Box, vec::Vec};
use log::debug;
use structure::{
    ConfigurationDescriptor, EndpointDescriptor, HidDescriptor, InterfaceDescriptor,
    CONFIGURATION_DESCRIPTOR_TYPE, ENDPOINT_DESCRIPTOR_TYPE, HID_DESCRIPTOR_TYPE,
    INTERFACE_DESCRIPTOR_TYPE,
};

use crate::device::xhc::descriptor;

use super::{driver::ClassDriverOperate, endpoint::EndpointConfig};

pub mod structure;

pub enum Descriptor {
    Configuration(ConfigurationDescriptor),
    Interface(InterfaceDescriptor),
    Endpoint(EndpointDescriptor),
    Hid(HidDescriptor),
    NotSupport,
}

impl Descriptor {
    pub fn interface(&self) -> Option<&InterfaceDescriptor> {
        if let Self::Interface(descriptor) = self {
            Some(descriptor)
        } else {
            None
        }
    }
}

pub struct DescriptorIterator {
    ptr: *mut u8,
    index: usize,
    len: usize,
}

impl DescriptorIterator {
    pub fn new(ptr: *mut u8, len: usize) -> Self {
        Self { ptr, index: 0, len }
    }
}

impl Iterator for DescriptorIterator {
    type Item = Descriptor;

    fn next(&mut self) -> Option<Self::Item> {
        if self.len <= self.index {
            return None;
        }

        let ptr = unsafe { self.ptr.add(self.index) };
        let (size, descriptor) = unsafe { convert_to_descriptor(ptr) };
        self.index += size;

        Some(descriptor)
    }
}

unsafe fn convert_to_descriptor(ptr: *mut u8) -> (usize, Descriptor) {
    let descriptor_type = *ptr.add(1);

    fn convert<T>(ptr: *mut u8) -> (usize, T) {
        (size_of::<T>(), unsafe { (ptr as *const T).read_volatile() })
    }

    match descriptor_type {
        CONFIGURATION_DESCRIPTOR_TYPE => {
            let (size, descriptor) = convert::<ConfigurationDescriptor>(ptr);
            (size, Descriptor::Configuration(descriptor))
        }
        INTERFACE_DESCRIPTOR_TYPE => {
            let (size, descriptor) = convert::<InterfaceDescriptor>(ptr);
            (size, Descriptor::Interface(descriptor))
        }
        ENDPOINT_DESCRIPTOR_TYPE => {
            let (size, descriptor) = convert::<EndpointDescriptor>(ptr);
            (size, Descriptor::Endpoint(descriptor))
        }
        HID_DESCRIPTOR_TYPE => {
            let (size, descriptor) = convert::<HidDescriptor>(ptr);
            (size, Descriptor::Hid(descriptor))
        }
        _ => (0, Descriptor::NotSupport),
    }
}

pub struct HidDeviceDescriptors {
    interface: InterfaceDescriptor,
    endpoint: EndpointDescriptor,
}

impl HidDeviceDescriptors {
    pub fn new(interface: InterfaceDescriptor, endpoint: EndpointDescriptor) -> Self {
        Self {
            interface,
            endpoint,
        }
    }

    pub fn class_driver(
        &self,
        drivers: &Vec<Box<dyn ClassDriverOperate>>,
    ) -> Option<Box<dyn ClassDriverOperate>> {
        let driver_type = self.interface.driver_type();
        for driver in drivers.iter() {
            if driver.driver_type() == driver_type {
                debug!("[XHCI]: {driver_type:?} Found");
                return Some(driver.clone());
            }
        }
        None
    }

    pub fn interface(&self) -> InterfaceDescriptor {
        self.interface.clone()
    }

    pub fn endpoint_config(&self) -> EndpointConfig {
        EndpointConfig::new(&self.endpoint)
    }
}
