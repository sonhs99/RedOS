use modular_bitfield::bitfield;
use modular_bitfield::prelude::{B2, B3, B4};

use crate::device::xhc::driver::DriverType;

pub(crate) const CONFIGURATION_DESCRIPTOR_TYPE: u8 = 2;
pub const INTERFACE_DESCRIPTOR_TYPE: u8 = 4;
pub const ENDPOINT_DESCRIPTOR_TYPE: u8 = 5;
pub(crate) const HID_DESCRIPTOR_TYPE: u8 = 33;

#[repr(packed)]
#[derive(Debug, Copy, Clone)]
pub struct ConfigurationDescriptor {
    pub length: u8,
    pub descriptor_type: u8,
    pub total_length: u16,
    pub num_interfaces: u8,
    pub configuration_value: u8,
    pub configuration_id: u8,
    pub attributes: u8,
    pub max_power: u8,
}

#[repr(packed)]
#[derive(Debug, Copy, Clone)]
pub struct DeviceDescriptor {
    pub length: u8,
    pub descriptor_type: u8,
    pub usb_release: u16,
    pub device_class: u8,
    pub device_sub_class: u8,
    pub device_protocol: u8,
    pub max_packet_size: u8,
    pub vendor_id: u16,
    pub product_id: u16,
    pub device_release: u16,
    pub manufacturer: u8,
    pub product: u8,
    pub serial_number: u8,
    pub num_configurations: u8,
}

#[bitfield]
#[derive(Debug, Clone)]
pub struct EndpointDescriptor {
    pub length: u8,
    pub descriptor_type: u8,
    pub endpoint_address: EndpointAddress,
    pub attributes: Attributes,
    pub max_packet_size: u16,
    pub interval: u8,
}

#[bitfield]
#[derive(Debug, BitfieldSpecifier)]
pub struct EndpointAddress {
    pub number: B4,
    #[skip]
    reserve: B3,
    pub dir_in: bool,
}

#[bitfield]
#[derive(Debug, BitfieldSpecifier)]
pub struct Attributes {
    pub transfer_type: B2,
    pub sync_type: B2,
    pub usage_type: B2,
    #[skip]
    reserve: B2,
}

#[bitfield(bits = 72)]
#[derive(Debug)]
pub struct HidDescriptor {
    pub length: u8,
    pub descriptor_type: u8,
    pub hid_release: u16,
    pub country_code: u8,
    pub num_descriptors: u8,
    pub class_descriptor: ClassDescriptor,
}

#[bitfield(bits = 24)]
#[derive(Debug, BitfieldSpecifier)]
pub struct ClassDescriptor {
    pub descriptor_type: u8,
    pub descriptor_length: u16,
}

#[derive(Debug, Clone)]
#[repr(packed)]
pub struct InterfaceDescriptor {
    pub length: u8,
    pub descriptor_type: u8,
    pub interface_number: u8,
    pub alternate_setting: u8,
    pub num_endpoints: u8,
    pub interface_class: u8,
    pub interface_sub_class: u8,
    pub interface_protocol: u8,
    pub interface_id: u8,
}

impl InterfaceDescriptor {
    pub fn is_mouse(&self) -> bool {
        self.interface_class == 3 && self.interface_sub_class == 1 && self.interface_protocol == 2
    }
    pub fn is_keyboard(&self) -> bool {
        self.interface_class == 3 && self.interface_sub_class == 1 && self.interface_protocol == 1
    }

    pub fn driver_type(&self) -> DriverType {
        if self.interface_class == 3
            && self.interface_sub_class == 1
            && self.interface_protocol == 1
        {
            DriverType::Keyboard
        } else if self.interface_class == 3
            && self.interface_sub_class == 1
            && self.interface_protocol == 2
        {
            DriverType::Mouse
        } else {
            DriverType::Unknown
        }
    }
}
