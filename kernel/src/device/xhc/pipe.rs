use alloc::rc::Rc;
use core::cell::RefCell;
use log::debug;
use xhci::ring::trb::transfer::{DataStage, Direction, SetupStage, StatusStage, TransferType};

use super::{device::DeviceContextIndex, register::DoorbellRegisterAccessible, ring::TransferRing};

use modular_bitfield::bitfield;
use modular_bitfield::prelude::{B2, B5};

pub enum Request {
    GetDescriptor(SetupStage),
    Configuration(SetupStage),
    SetProtocol(SetupStage),
    GetReport(SetupStage),
}

#[bitfield]
#[repr(u8)]
#[derive(Debug, Clone)]
pub struct RequestType {
    pub recipient: B5,
    pub ty: B2,
    pub direction: bool,
}

impl RequestType {
    pub fn raw(&self) -> u8 {
        self.clone().into_bytes()[0]
    }
}

impl Request {
    pub fn get_descriptor(desc_type: u16, desc_index: u16, len: u16) -> Self {
        let mut setup_data = SetupStage::new();
        const GET_DESCRIPTOR: u8 = 6;
        setup_data.set_request_type(RequestType::new().with_direction(true).raw());
        setup_data.set_request(GET_DESCRIPTOR);
        setup_data.set_value(desc_type << 8 | desc_index);
        setup_data.set_index(0);
        setup_data.set_length(len);
        Self::GetDescriptor(setup_data)
    }

    pub fn get_report(report_len: u16, interface_number: u16) -> Self {
        let mut setup_data = SetupStage::new();
        const GET_REPORT: u8 = 1;
        setup_data.set_request_type(
            RequestType::new()
                .with_direction(true)
                .with_ty(1)
                .with_recipient(1)
                .raw(),
        );
        setup_data.set_request(GET_REPORT);
        setup_data.set_value(0x100);
        setup_data.set_index(interface_number);
        setup_data.set_transfer_type(TransferType::In);
        Self::GetReport(setup_data)
    }

    pub fn configuration(config_value: u16) -> Self {
        let mut setup_data = SetupStage::new();
        const CONFIGURATION: u8 = 9;
        setup_data.set_request(CONFIGURATION);
        setup_data.set_value(config_value);
        setup_data.set_index(0);
        setup_data.set_length(0);
        Self::Configuration(setup_data)
    }

    pub fn set_protocol(request_type: RequestType, interface_num: u16) -> Self {
        let mut setup = SetupStage::new();

        setup.set_interrupt_on_completion();
        setup.set_index(interface_num);
        setup.set_value(0);
        setup.set_request_type(request_type.raw());
        setup.set_request(11);
        setup.set_length(0);

        Self::SetProtocol(setup)
    }

    pub fn setup_stage(&self) -> SetupStage {
        match self {
            Request::GetDescriptor(setup) => *setup,
            Request::Configuration(setup) => *setup,
            Request::SetProtocol(setup) => *setup,
            Request::GetReport(setup) => *setup,
        }
    }
}

pub(crate) fn make_setup(setup_stage: SetupStage, transfer_type: TransferType) -> SetupStage {
    let mut setup_data = SetupStage::new();
    setup_data.set_request_type(setup_stage.request_type());
    setup_data.set_request(setup_stage.request());
    setup_data.set_value(setup_stage.value());
    setup_data.set_index(setup_stage.index());
    setup_data.set_length(setup_stage.length());
    setup_data.set_transfer_type(transfer_type);
    setup_data
}

pub(crate) fn make_data(data_buff: u64, length: u32, direction: Direction) -> DataStage {
    let mut data_stage = DataStage::new();

    data_stage.set_data_buffer_pointer(data_buff);
    data_stage.set_trb_transfer_length(length);
    data_stage.set_td_size(0);
    data_stage.set_direction(direction);

    data_stage
}

pub trait ControlPipeTransfer {
    fn no_data(&mut self, request: Request) -> Result<(), ()>;
    fn with_data(&mut self, request: Request, data_buff_addr: u64, len: u32) -> Result<(), ()>;
}

pub struct ControlIn<T> {
    slot_id: u8,
    device_context_index: DeviceContextIndex,
    doorbell: Rc<RefCell<T>>,
    transfer_ring: Rc<RefCell<TransferRing>>,
}

impl<T> ControlIn<T>
where
    T: DoorbellRegisterAccessible,
{
    pub fn new(
        slot_id: u8,
        device_context_index: DeviceContextIndex,
        doorbell: &Rc<RefCell<T>>,
        transfer_ring: &Rc<RefCell<TransferRing>>,
    ) -> Self {
        Self {
            slot_id,
            device_context_index,
            doorbell: doorbell.clone(),
            transfer_ring: transfer_ring.clone(),
        }
    }

    pub fn notify(&mut self) -> Result<(), ()> {
        // debug!("ring the doorbell in");
        self.doorbell.borrow_mut().notify_at(
            self.slot_id as usize,
            self.device_context_index.as_u8(),
            0,
        )
    }

    fn push(&mut self, trb: [u32; 4]) -> Result<(), ()> {
        self.transfer_ring.borrow_mut().push(trb)
    }
}

pub struct ControlOut<T> {
    slot_id: u8,
    device_context_index: DeviceContextIndex,
    doorbell: Rc<RefCell<T>>,
    transfer_ring: Rc<RefCell<TransferRing>>,
}

impl<T> ControlOut<T>
where
    T: DoorbellRegisterAccessible,
{
    pub fn new(
        slot_id: u8,
        device_context_index: DeviceContextIndex,
        doorbell: &Rc<RefCell<T>>,
        transfer_ring: &Rc<RefCell<TransferRing>>,
    ) -> Self {
        Self {
            slot_id,
            device_context_index,
            doorbell: doorbell.clone(),
            transfer_ring: transfer_ring.clone(),
        }
    }

    pub fn notify(&mut self) -> Result<(), ()> {
        // debug!("ring the doorbell out");
        self.doorbell.borrow_mut().notify_at(
            self.slot_id as usize,
            self.device_context_index.as_u8(),
            0,
        )
    }

    fn push(&mut self, trb: [u32; 4]) -> Result<(), ()> {
        self.transfer_ring.borrow_mut().push(trb)
    }
}

impl<T> ControlPipeTransfer for ControlIn<T>
where
    T: DoorbellRegisterAccessible,
{
    fn no_data(&mut self, request: Request) -> Result<(), ()> {
        let setup = make_setup(request.setup_stage(), TransferType::No);
        self.push(setup.into_raw())?;

        let mut status = StatusStage::new();
        status.set_direction();
        status.set_interrupt_on_completion();
        self.push(status.into_raw())?;

        self.notify()
    }

    fn with_data(&mut self, request: Request, data_buff_addr: u64, len: u32) -> Result<(), ()> {
        let setup = make_setup(request.setup_stage(), TransferType::In);
        self.push(setup.into_raw())?;

        let mut data = make_data(data_buff_addr, len, Direction::In);
        data.set_interrupt_on_completion();
        data.set_interrupt_on_short_packet();
        self.push(data.into_raw())?;

        self.push(StatusStage::new().into_raw())?;

        self.notify()
    }
}

impl<T> ControlPipeTransfer for ControlOut<T>
where
    T: DoorbellRegisterAccessible,
{
    fn no_data(&mut self, request: Request) -> Result<(), ()> {
        let setup = make_setup(request.setup_stage(), TransferType::No);
        self.push(setup.into_raw())?;

        let mut status = StatusStage::new();
        status.set_direction();
        status.set_interrupt_on_completion();
        self.push(status.into_raw())?;

        self.notify()
    }

    fn with_data(&mut self, request: Request, data_buff_addr: u64, len: u32) -> Result<(), ()> {
        let setup = make_setup(request.setup_stage(), TransferType::Out);
        self.push(setup.into_raw())?;

        let mut data = make_data(data_buff_addr, len, Direction::Out);
        data.set_interrupt_on_completion();
        data.set_interrupt_on_short_packet();
        self.push(data.into_raw())?;

        let mut status = StatusStage::new();
        status.set_direction();
        self.push(status.into_raw())?;

        self.notify()
    }
}

pub struct ControlPipe<T> {
    transfer_ring: Rc<RefCell<TransferRing>>,
    control_in: ControlIn<T>,
    control_out: ControlOut<T>,
}

impl<T> ControlPipe<T>
where
    T: DoorbellRegisterAccessible,
{
    pub fn new(
        slot_id: u8,
        device_context_index: DeviceContextIndex,
        doorbell: &Rc<RefCell<T>>,
        transfer_ring: TransferRing,
    ) -> Result<Self, ()> {
        let transfer_ring = Rc::new(RefCell::new(transfer_ring));
        let control_in = ControlIn::new(slot_id, device_context_index, doorbell, &transfer_ring);
        let control_out = ControlOut::new(slot_id, device_context_index, doorbell, &transfer_ring);
        Ok(Self {
            transfer_ring,
            control_in,
            control_out,
        })
    }

    pub fn control_in(&mut self) -> &mut ControlIn<T> {
        &mut self.control_in
    }

    pub fn control_out(&mut self) -> &mut ControlOut<T> {
        &mut self.control_out
    }

    pub fn transfer_ring_base_addr(&self) -> u64 {
        self.transfer_ring.borrow().base_address()
    }
}
