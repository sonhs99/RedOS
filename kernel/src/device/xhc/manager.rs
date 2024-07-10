use core::cell::RefCell;

use alloc::{boxed::Box, rc::Rc, vec::Vec};
use log::{debug, error};
use xhci::ring::trb::event::TransferEvent;

use super::{
    allocator::Allocatable,
    device::{Device, DeviceConfig, DeviceMap, DeviceSlot},
    driver::ClassDriverOperate,
    event::TargetEvent,
    register::{
        DoorbellRegisterAccessible, OperationalRegsisterAccessible, PortRegisterAccessible,
    },
};

pub struct Manager<D, A> {
    devices: DeviceMap<D, A>,
    device_context_array: &'static mut [u64],
    addressing_port_id: Option<u8>,
    registers: Rc<RefCell<D>>,
    drivers: Vec<Box<dyn ClassDriverOperate>>,
}

impl<D, A> Manager<D, A>
where
    D: DoorbellRegisterAccessible + PortRegisterAccessible + 'static,
    A: Allocatable,
{
    pub fn new(
        devices: DeviceMap<D, A>,
        device_context_array: &'static mut [u64],
        registers: &Rc<RefCell<D>>,
        drivers: Vec<Box<dyn ClassDriverOperate>>,
    ) -> Manager<D, A> {
        Self {
            devices,
            device_context_array,
            addressing_port_id: None,
            registers: registers.clone(),
            drivers,
        }
    }

    pub fn is_addressing_port(&self, port_id: u8) -> bool {
        if let Some(id) = self.addressing_port_id {
            port_id == id
        } else {
            true
        }
    }

    pub fn set_addressing_port(&mut self, port_id: u8) {
        self.addressing_port_id = Some(port_id);
    }

    pub fn device_slot_at(&mut self, slot_id: u8) -> Result<&mut Device<D, A>, ()> {
        self.devices.get_mut(slot_id)
    }

    pub fn address_device(&mut self, slot_id: u8, allocator: &Rc<RefCell<A>>) -> Result<u64, ()> {
        let parent_hub_slot_id = self.try_addressing_id_port()?;

        let device = self.new_device(parent_hub_slot_id, slot_id, allocator)?;
        let device_context_addr = device.device_context_addr();
        let input_context_addr = device.input_context_addr();

        self.device_context_array[slot_id as usize] = device_context_addr;

        // unsafe {
        //     *(self.device_context_array.as_mut_ptr() as *mut u64).add(slot_id as usize) =
        //         device_context_addr
        // };

        self.addressing_port_id = None;

        Ok(input_context_addr)
    }

    pub fn initialize_at(&mut self, slot_id: u8) -> Result<(), ()> {
        self.device_mut_at(slot_id)?.initialize()
    }

    pub fn process_transfer_event(
        &mut self,
        slot_id: u8,
        transfer_event: TransferEvent,
        target_event: TargetEvent,
    ) -> Result<bool, ()> {
        let device = self.device_mut_at(slot_id)?;
        let init_status = device.on_transfer_event_received(transfer_event, target_event)?;
        Ok(init_status.is_initialized())
    }

    pub fn configure_endpoint(&mut self, slot_id: u8) -> Result<(), ()> {
        self.device_mut_at(slot_id)?.on_endpoints_configured()
    }

    fn try_addressing_id_port(&self) -> Result<u8, ()> {
        match self.addressing_port_id {
            Some(port) => Ok(port),
            None => Err(error!("Not Exists Addressing Port")),
        }
    }

    fn new_device(
        &mut self,
        parent_hub_slot_id: u8,
        slot_id: u8,
        allocator: &Rc<RefCell<A>>,
    ) -> Result<&mut Device<D, A>, ()> {
        let port_speed = self
            .registers
            .borrow()
            .read_port_speed_at(parent_hub_slot_id)?;
        let config = DeviceConfig::new(parent_hub_slot_id, port_speed, slot_id);

        self.devices
            .new_set(config, allocator, &self.registers, self.drivers.clone())
    }

    fn device_mut_at(&mut self, slot_id: u8) -> Result<&mut Device<D, A>, ()> {
        self.devices.get_mut(slot_id)
    }
}

pub(crate) fn make_manager<T, A>(
    registers: &mut Rc<RefCell<T>>,
    device_slots: u8,
    scratchpad_buffer_len: usize,
    allocator: &mut impl Allocatable,
    drivers: Vec<Box<dyn ClassDriverOperate>>,
) -> Result<Manager<T, A>, ()>
where
    T: DoorbellRegisterAccessible
        + PortRegisterAccessible
        + OperationalRegsisterAccessible
        + 'static,
    A: Allocatable,
{
    let device_context_array = registers.borrow_mut().setup_device_context_array(
        device_slots,
        scratchpad_buffer_len,
        allocator,
    )?;

    Ok(Manager::new(
        DeviceMap::default(),
        device_context_array,
        registers,
        drivers,
    ))
}
