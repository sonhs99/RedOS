use core::cell::RefCell;

use alloc::{boxed::Box, collections::BTreeMap, rc::Rc, vec::Vec};
use log::{debug, error};
use xhci::{context::EndpointType, ring::trb::event::TransferEvent};

use crate::device::xhc::pipe::{ControlPipeTransfer, Request};

use super::{
    allocator::Allocatable,
    context::{DeviceContext, InputContext},
    driver::ClassDriverOperate,
    endpoint::EndpointId,
    event::TargetEvent,
    phase::{InitState, Phase, Phase1},
    pipe::{ControlPipe, RequestType},
    register::DoorbellRegisterAccessible,
    ring::TransferRing,
};

const DATA_BUFF_SIZE: usize = 256;

const RING_LENGTH: usize = 32;

pub struct DeviceSlot<D, A> {
    slot_id: u8,
    default_control_pipe: ControlPipe<D>,
    input_context: InputContext,
    device_context: DeviceContext,
    data_buff: [u8; DATA_BUFF_SIZE],
    doorbell: Rc<RefCell<D>>,
    allocator: Rc<RefCell<A>>,
}

impl<D, A> DeviceSlot<D, A>
where
    D: DoorbellRegisterAccessible,
    A: Allocatable,
{
    pub fn new(
        slot_id: u8,
        doorbell: &Rc<RefCell<D>>,
        allocator: &Rc<RefCell<A>>,
    ) -> Result<Self, ()> {
        let transfer_ring = allocator
            .borrow_mut()
            .alloc_array::<u128>(RING_LENGTH, 64, 4096)?;
        let transfer_ring = TransferRing::new(transfer_ring.as_ptr() as u64, RING_LENGTH, true);
        let default_control_pipe = ControlPipe::new(
            slot_id,
            DeviceContextIndex::default(),
            doorbell,
            transfer_ring,
        )?;

        Ok(Self {
            slot_id,
            default_control_pipe,
            data_buff: [0u8; DATA_BUFF_SIZE],
            input_context: InputContext::new(),
            device_context: DeviceContext::new(),
            doorbell: doorbell.clone(),
            allocator: allocator.clone(),
        })
    }

    pub fn id(&self) -> u8 {
        self.slot_id
    }

    pub fn data_buff_addr(&self) -> u64 {
        self.data_buff.as_ptr() as u64
    }

    pub fn data_buff_len(&self) -> usize {
        self.data_buff.len()
    }

    pub fn input_context(&self) -> &InputContext {
        &self.input_context
    }

    pub fn input_context_mut(&mut self) -> &mut InputContext {
        &mut self.input_context
    }

    pub fn device_context(&self) -> &DeviceContext {
        &self.device_context
    }

    pub fn copy_device_context_to_input(&mut self) {
        self.input_context
            .copy_from_device_context(self.device_context.slot())
    }

    pub fn default_control_pipe(&self) -> &ControlPipe<D> {
        &self.default_control_pipe
    }

    pub fn default_control_pipe_mut(&mut self) -> &mut ControlPipe<D> {
        &mut self.default_control_pipe
    }

    pub fn doorbell(&self) -> &Rc<RefCell<D>> {
        &self.doorbell
    }

    pub fn try_alloc_transfer_ring(&mut self, ring_size: usize) -> Result<TransferRing, ()> {
        let transfer_ring_addr =
            self.allocator
                .borrow_mut()
                .alloc_array::<u128>(RING_LENGTH, 64, 4096)?;
        Ok(TransferRing::new(
            transfer_ring_addr.as_ptr() as u64,
            ring_size,
            true,
        ))
    }
}

#[derive(Debug, Clone, Copy)]
pub struct DeviceConfig {
    parent_hub_slot_id: u8,
    port_speed: u8,
    slot_id: u8,
}

impl DeviceConfig {
    pub fn new(parent_hub_slot_id: u8, port_speed: u8, slot_id: u8) -> Self {
        Self {
            parent_hub_slot_id,
            port_speed,
            slot_id,
        }
    }

    pub const fn parent_hub_slot_id(&self) -> u8 {
        self.parent_hub_slot_id
    }

    pub const fn port_speed(&self) -> u8 {
        self.port_speed
    }

    pub const fn slot_id(&self) -> u8 {
        self.slot_id
    }
}

pub struct Device<D, A> {
    slot_id: u8,
    phase: Box<dyn Phase<D, A>>,
    slot: DeviceSlot<D, A>,
    device_descriptor_buff: [u8; DATA_BUFF_SIZE],
}

impl<D, A> Device<D, A>
where
    D: DoorbellRegisterAccessible + 'static,
    A: Allocatable,
{
    pub fn new(
        slot_id: u8,
        allocator: &Rc<RefCell<A>>,
        doorbell: &Rc<RefCell<D>>,
        drivers: Vec<Box<dyn ClassDriverOperate>>,
    ) -> Result<Self, ()> {
        Ok(Self {
            slot_id,
            slot: DeviceSlot::new(slot_id, doorbell, allocator)?,
            phase: Box::new(Phase1::new(drivers)),
            device_descriptor_buff: [0u8; DATA_BUFF_SIZE],
        })
    }
    pub fn device_context_addr(&self) -> u64 {
        self.slot.device_context().device_context_addr()
    }

    pub fn input_context_addr(&self) -> u64 {
        self.slot.input_context().input_context_addr()
    }

    pub fn slot_id(&self) -> u8 {
        self.slot_id
    }

    fn init_slot_context(&mut self, root_port_hub_id: u8, port_speed: u8) {
        let input_context = self.slot.input_context_mut();
        let slot = input_context.slot_mut();
        slot.set_root_hub_port_number(root_port_hub_id);
        slot.set_route_string(0);
        slot.set_context_entries(1);
        slot.set_speed(port_speed);
    }

    fn init_default_control_pipe(&mut self, port_speed: u8) {
        let tr_dequeue_addr = self.slot.default_control_pipe().transfer_ring_base_addr();
        let control = self.slot.input_context_mut();
        let default_control_pipe = control.endpoint_mut_at(DeviceContextIndex::default().value());

        default_control_pipe.set_endpoint_type(EndpointType::Control);
        default_control_pipe.set_max_packet_size(max_packet_size(port_speed));
        default_control_pipe.set_max_burst_size(0);
        default_control_pipe.set_tr_dequeue_pointer(tr_dequeue_addr);
        default_control_pipe.set_dequeue_cycle_state();
        default_control_pipe.set_interval(0);
        default_control_pipe.set_max_primary_streams(0);
        default_control_pipe.set_mult(0);
        default_control_pipe.set_error_count(3);
    }

    pub fn new_with_init_default_control_pipe(
        config: DeviceConfig,
        allocator: &Rc<RefCell<A>>,
        doorbell: &Rc<RefCell<D>>,
        drivers: Vec<Box<dyn ClassDriverOperate>>,
    ) -> Result<Self, ()> {
        let mut me = Self::new(config.slot_id(), allocator, doorbell, drivers)?;
        me.slot.input_context_mut().set_enable_slot_context();
        me.slot
            .input_context_mut()
            .set_enable_endpoint(DeviceContextIndex::default());
        me.init_slot_context(config.parent_hub_slot_id(), config.port_speed());
        me.init_default_control_pipe(config.port_speed());
        Ok(me)
    }

    pub fn initialize(&mut self) -> Result<(), ()> {
        let buff = self.device_descriptor_buff.as_mut_ptr();
        const DEVICE_DESCRIPTOR_TYPE: u16 = 1;
        let data_buff_addr = buff as u64;
        let len = self.device_descriptor_buff.len() as u32;
        self.slot.default_control_pipe_mut().control_in().with_data(
            Request::get_descriptor(DEVICE_DESCRIPTOR_TYPE, 0, len as u16),
            data_buff_addr,
            len,
        )
    }

    pub fn on_transfer_event_received(
        &mut self,
        transfer_event: TransferEvent,
        target_event: TargetEvent,
    ) -> Result<InitState, ()> {
        let (init_state, phase) =
            self.phase
                .on_transfer_event_received(&mut self.slot, transfer_event, target_event)?;

        if let Some(phase) = phase {
            self.phase = phase;
        }

        Ok(init_state)
    }

    pub fn on_endpoints_configured(&mut self) -> Result<(), ()> {
        for interface_num in self.phase.interface_nums().unwrap() {
            let request_type = RequestType::new().with_ty(1).with_recipient(1);
            self.slot
                .default_control_pipe_mut()
                .control_out()
                .no_data(Request::set_protocol(request_type, interface_num as u16))?;
        }

        Ok(())
    }
}

pub struct DeviceMap<D, A> {
    map: BTreeMap<u8, Device<D, A>>,
}

impl<D, A> DeviceMap<D, A>
where
    D: DoorbellRegisterAccessible + 'static,
    A: Allocatable,
{
    pub fn new_set(
        &mut self,
        config: DeviceConfig,
        allocator: &Rc<RefCell<A>>,
        doorbell: &Rc<RefCell<D>>,
        drivers: Vec<Box<dyn ClassDriverOperate>>,
    ) -> Result<&mut Device<D, A>, ()> {
        self.set(Device::new_with_init_default_control_pipe(
            config, allocator, doorbell, drivers,
        )?);
        self.get_mut(config.slot_id())
    }

    fn set(&mut self, device: Device<D, A>) {
        self.map.insert(device.slot_id, device);
    }

    pub fn get_mut(&mut self, slot_id: u8) -> Result<&mut Device<D, A>, ()> {
        match self.map.get_mut(&slot_id) {
            Some(device) => Ok(device),
            None => Err(error!("Not Found Device id={slot_id}")),
        }
    }
}

impl<D, A> Default for DeviceMap<D, A> {
    fn default() -> Self {
        Self {
            map: BTreeMap::default(),
        }
    }
}

#[derive(Clone, Copy)]
pub struct DeviceContextIndex(usize);

impl Default for DeviceContextIndex {
    fn default() -> Self {
        Self(1)
    }
}

impl DeviceContextIndex {
    pub fn from_endpoint(endpoint_id: EndpointId) -> Self {
        Self(endpoint_id.value())
    }

    pub fn from_dci(dci: usize) -> Self {
        Self(dci)
    }

    pub fn new(endpoint_num: usize, is_control_in: bool) -> Self {
        Self(2 * endpoint_num + (endpoint_num == 0 || is_control_in) as usize)
    }

    pub fn value(&self) -> usize {
        self.0
    }

    pub fn as_u8(&self) -> u8 {
        self.0 as u8
    }
}

fn max_packet_size(port_speed: u8) -> u16 {
    match port_speed {
        3 => 64,
        4 => 512,
        _ => 8,
    }
}
