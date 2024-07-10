use alloc::{boxed::Box, vec::Vec};
use log::debug;
use xhci::ring::trb::event::TransferEvent;

use crate::device::xhc::pipe::{ControlPipe, Request};

use super::{
    allocator::Allocatable,
    descriptor::{
        structure::{ConfigurationDescriptor, HidDescriptor, InterfaceDescriptor},
        Descriptor, DescriptorIterator, HidDeviceDescriptors,
    },
    device::{DeviceContextIndex, DeviceSlot},
    driver::{ClassDriverOperate, DriverType, InterruptIn},
    event::TargetEvent,
    pipe::ControlPipeTransfer,
    register::DoorbellRegisterAccessible,
};

pub(crate) const DATA_BUFF_SIZE: usize = 256;

pub struct InitState(bool);

impl InitState {
    pub fn new(is_initialized: bool) -> Self {
        Self(is_initialized)
    }

    pub fn initialized() -> Self {
        Self::new(true)
    }

    pub fn not_initialized() -> Self {
        Self::new(false)
    }

    pub fn is_initialized(&self) -> bool {
        self.0
    }
}

pub trait Phase<D, A>
where
    D: DoorbellRegisterAccessible,
    A: Allocatable,
{
    fn on_transfer_event_received(
        &mut self,
        slot: &mut DeviceSlot<D, A>,
        transfer_event: TransferEvent,
        target_event: TargetEvent,
    ) -> Result<(InitState, Option<Box<dyn Phase<D, A>>>), ()>;
    fn interface_nums(&self) -> Option<Vec<u8>>;
}

pub struct Phase1 {
    drivers: Vec<Box<dyn ClassDriverOperate>>,
}

impl Phase1 {
    pub fn new(drivers: Vec<Box<dyn ClassDriverOperate>>) -> Self {
        Self { drivers }
    }
}

impl<D, A> Phase<D, A> for Phase1
where
    D: DoorbellRegisterAccessible + 'static,
    A: Allocatable,
{
    fn on_transfer_event_received(
        &mut self,
        slot: &mut DeviceSlot<D, A>,
        transfer_event: TransferEvent,
        target_event: TargetEvent,
    ) -> Result<(InitState, Option<Box<dyn Phase<D, A>>>), ()> {
        const CONFIGURATION_TYPE: u16 = 2;

        let data_buff_addr = slot.data_buff_addr();
        let len = slot.data_buff_len() as u32;
        let request = Request::get_descriptor(CONFIGURATION_TYPE, 0, len as u16);
        slot.default_control_pipe_mut()
            .control_in()
            .with_data(request, data_buff_addr, len)?;

        Ok((
            InitState::not_initialized(),
            Some(Box::new(Phase2::new(self.drivers.clone()))),
        ))
    }

    fn interface_nums(&self) -> Option<Vec<u8>> {
        None
    }
}

pub struct Phase2 {
    drivers: Vec<Box<dyn ClassDriverOperate>>,
}

impl Phase2 {
    pub fn new(drivers: Vec<Box<dyn ClassDriverOperate>>) -> Self {
        Self { drivers }
    }
}

impl<D, A> Phase<D, A> for Phase2
where
    D: DoorbellRegisterAccessible + 'static,
    A: Allocatable,
{
    fn on_transfer_event_received(
        &mut self,
        slot: &mut DeviceSlot<D, A>,
        transfer_event: TransferEvent,
        target_event: TargetEvent,
    ) -> Result<(InitState, Option<Box<dyn Phase<D, A>>>), ()> {
        let data_stage = target_event.data_stage()?;

        let conf_desc_buff = data_stage.data_buffer_pointer() as *mut u8;
        let conf_desc_buff_len =
            (data_stage.trb_transfer_length() - transfer_event.trb_transfer_length()) as usize;

        let conf_desc = unsafe { *conf_desc_buff.cast::<ConfigurationDescriptor>() };
        let descriptors = DescriptorIterator::new(conf_desc_buff, conf_desc_buff_len)
            .collect::<Vec<Descriptor>>();

        let hid_desc: Vec<HidDeviceDescriptors> = descriptors
            .iter()
            .enumerate()
            .filter_map(filter_interface)
            .filter(|(index, interface)| interface.driver_type() != DriverType::Unknown)
            .filter_map(|(index, interface)| map_hid_descriptors(index, interface, &descriptors))
            .collect();

        slot.input_context_mut()
            .set_config(conf_desc.configuration_value);

        set_configuration(
            conf_desc.configuration_value as u16,
            slot.default_control_pipe_mut(),
        )?;

        Ok((
            InitState::not_initialized(),
            Some(Box::new(Phase3::new(self.drivers.clone(), hid_desc))),
        ))
    }

    fn interface_nums(&self) -> Option<Vec<u8>> {
        None
    }
}

fn set_configuration<T: DoorbellRegisterAccessible>(
    config_value: u16,
    default_control_pipe: &mut ControlPipe<T>,
) -> Result<(), ()> {
    default_control_pipe
        .control_out()
        .no_data(Request::configuration(config_value))
}

fn filter_interface((index, device): (usize, &Descriptor)) -> Option<(usize, InterfaceDescriptor)> {
    device
        .interface()
        .map(|interface| (index, interface.clone()))
}

fn map_hid_descriptors(
    index: usize,
    interface: InterfaceDescriptor,
    descriptors: &[Descriptor],
) -> Option<HidDeviceDescriptors> {
    let endpoint = descriptors
        .iter()
        .skip(index + 1 + 1)
        .find_map(|descriptor| {
            if let Descriptor::Endpoint(endpoint) = descriptor {
                Some(endpoint)
            } else {
                None
            }
        })?;
    Some(HidDeviceDescriptors::new(interface, endpoint.clone()))
}

pub struct Phase3 {
    drivers: Vec<Box<dyn ClassDriverOperate>>,
    hid_device_descriptor_vec: Vec<HidDeviceDescriptors>,
}

impl Phase3 {
    pub fn new(
        drivers: Vec<Box<dyn ClassDriverOperate>>,
        hid_device_descriptor_vec: Vec<HidDeviceDescriptors>,
    ) -> Self {
        Self {
            drivers,
            hid_device_descriptor_vec,
        }
    }

    fn interrupters<D, A>(&mut self, slot: &mut DeviceSlot<D, A>) -> Vec<InterruptIn<D>>
    where
        D: DoorbellRegisterAccessible,
        A: Allocatable,
    {
        self.hid_device_descriptor_vec
            .iter()
            .filter_map(|hid| {
                let class_driver = hid.class_driver(&self.drivers)?;
                let transfer_ring = slot.try_alloc_transfer_ring(32).ok()?;
                Some(InterruptIn::new(
                    slot.id(),
                    class_driver,
                    &hid.endpoint_config(),
                    transfer_ring,
                    hid.interface(),
                    slot.doorbell(),
                ))
            })
            .collect()
    }
}

impl<D, A> Phase<D, A> for Phase3
where
    D: DoorbellRegisterAccessible + 'static,
    A: Allocatable,
{
    fn on_transfer_event_received(
        &mut self,
        slot: &mut DeviceSlot<D, A>,
        transfer_event: TransferEvent,
        target_event: TargetEvent,
    ) -> Result<(InitState, Option<Box<dyn Phase<D, A>>>), ()> {
        slot.input_context_mut().clear_control();
        slot.copy_device_context_to_input();
        slot.input_context_mut().set_enable_slot_context();
        slot.input_context_mut().slot_mut().set_context_entries(31);

        let interrupters = self.interrupters(slot);
        interrupters.iter().for_each(|interrupter| {
            let config = interrupter.endpoint_config();
            let dci = DeviceContextIndex::from_endpoint(config.endpoint_id());

            slot.input_context_mut().set_enable_endpoint(dci);
            let endpoint_ctx = slot.input_context_mut().endpoint_mut_at(dci.value());

            config.write_endpoint_context(interrupter.transfer_ring_addr(), endpoint_ctx);
        });

        Ok((
            InitState::initialized(),
            Some(Box::new(Phase4::new(interrupters))),
        ))
    }

    fn interface_nums(&self) -> Option<Vec<u8>> {
        None
    }
}

pub struct Phase4<D>
where
    D: DoorbellRegisterAccessible,
{
    interrupters: Vec<InterruptIn<D>>,
}

impl<D> Phase4<D>
where
    D: DoorbellRegisterAccessible,
{
    pub fn new(interrupters: Vec<InterruptIn<D>>) -> Self {
        Self { interrupters }
    }
}

impl<D, A> Phase<D, A> for Phase4<D>
where
    D: DoorbellRegisterAccessible + 'static,
    A: Allocatable,
{
    fn on_transfer_event_received(
        &mut self,
        slot: &mut DeviceSlot<D, A>,
        transfer_event: TransferEvent,
        target_event: TargetEvent,
    ) -> Result<(InitState, Option<Box<dyn Phase<D, A>>>), ()> {
        for interrupter in self.interrupters.iter_mut() {
            interrupter.interrupt_in()?;
        }

        Ok((InitState::not_initialized(), None))
    }

    fn interface_nums(&self) -> Option<Vec<u8>> {
        Some(
            self.interrupters
                .iter()
                .map(|interrupter| interrupter.interface_ref().interface_id)
                .collect(),
        )
    }
}
