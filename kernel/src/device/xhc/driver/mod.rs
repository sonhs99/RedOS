use core::cell::RefCell;

use alloc::{boxed::Box, rc::Rc};
use dyn_clone::DynClone;
use xhci::ring::trb::transfer::Normal;

use super::{
    descriptor::structure::InterfaceDescriptor,
    endpoint::EndpointConfig,
    pipe::{ControlPipe, ControlPipeTransfer, Request},
    register::DoorbellRegisterAccessible,
    ring::TransferRing,
};

#[derive(PartialEq)]
pub enum DriverType {
    Keyboard,
    Mouse,
    Unknown,
}

pub trait ClassDriverOperate: DynClone {
    fn on_data_received(&mut self) -> Result<(), ()>;
    fn data_buffer_addr(&self) -> u64;
    fn data_buffer_len(&self) -> u32;
    fn driver_type(&self) -> DriverType;
}

dyn_clone::clone_trait_object!(ClassDriverOperate);

pub struct InterruptIn<T>
where
    T: DoorbellRegisterAccessible,
{
    slot_id: u8,
    class_driver: Box<dyn ClassDriverOperate>,
    endpoint_config: EndpointConfig,
    transfer_ring: TransferRing,
    interface: InterfaceDescriptor,
    doorbell: Rc<RefCell<T>>,
}

impl<T> InterruptIn<T>
where
    T: DoorbellRegisterAccessible,
{
    pub fn new(
        slot_id: u8,
        class_driver: Box<dyn ClassDriverOperate>,
        endpoint_config: &EndpointConfig,
        transfer_ring: TransferRing,
        interface: InterfaceDescriptor,
        doorbell: &Rc<RefCell<T>>,
    ) -> Self {
        Self {
            slot_id,
            class_driver,
            endpoint_config: endpoint_config.clone(),
            transfer_ring,
            interface,
            doorbell: doorbell.clone(),
        }
    }

    pub fn get_report<Doorbell>(
        &mut self,
        default_control_pipe: &mut ControlPipe<Doorbell>,
    ) -> Result<(), ()>
    where
        Doorbell: DoorbellRegisterAccessible,
    {
        self.class_driver.on_data_received()?;

        default_control_pipe.control_in().with_data(
            Request::get_report(3, 0),
            self.class_driver.data_buffer_addr(),
            self.class_driver.data_buffer_len(),
        )
    }

    pub fn interrupt_in(&mut self) -> Result<(), ()> {
        self.class_driver.on_data_received()?;

        let mut normal = Normal::new();
        normal.set_data_buffer_pointer(self.class_driver.data_buffer_addr());
        normal.set_trb_transfer_length(self.class_driver.data_buffer_len());
        normal.set_interrupt_on_completion();
        normal.set_interrupt_on_short_packet();

        self.transfer_ring.push(normal.into_raw());

        self.notify()
    }

    pub fn endpoint_config(&self) -> &EndpointConfig {
        &self.endpoint_config
    }

    pub fn interface_ref(&self) -> &InterfaceDescriptor {
        &self.interface
    }

    pub fn transfer_ring_addr(&self) -> u64 {
        self.transfer_ring.base_address()
    }

    pub fn data_buff_addr(&self) -> u64 {
        self.class_driver.data_buffer_addr()
    }

    #[inline(always)]
    fn notify(&mut self) -> Result<(), ()> {
        let endpoint_id = self.endpoint_config.endpoint_id().value();
        self.doorbell
            .borrow_mut()
            .notify_at(self.slot_id as usize, endpoint_id as u8, 0)
    }
}
