pub mod allocator;
mod context;
mod descriptor;
mod device;
pub mod driver;
mod endpoint;
mod event;
mod manager;
mod phase;
mod pipe;
mod port;
pub mod register;
mod ring;
mod trb;

use core::{
    cell::RefCell, hint::black_box, mem::size_of, num::NonZero, ptr, slice::from_raw_parts_mut,
};

use alloc::{boxed::Box, rc::Rc, vec::Vec};
use allocator::{Allocatable, Allocator};
use driver::ClassDriverOperate;
use event::TargetEvent;
use log::{debug, info};
use manager::{make_manager, Manager};
use port::WaitingPort;
use register::{
    CapabilityRegisterAccessible, ConfigRegisterAccessible, DoorbellRegisterAccessible, External,
    InterrupterSetRegisterAccessible, OperationalRegsisterAccessible, PortRegisterAccessible,
    RegisterOperation, UsbCommandRegisterAccessible, XhcRegisters,
};
use ring::{make_command_ring, make_event_ring, CommandRing, EventRing, EventTrb};
use trb::TrbRaw;
use xhci::ring::trb::event::{CommandCompletion, PortStatusChange, TransferEvent};

use crate::sync::{Mutex, OnceLock};

const DEVICE_SIZE: usize = 8;

pub struct Controller<R, A> {
    mmio_base: u64,
    registers: Rc<RefCell<R>>,
    device_manager: Manager<R, A>,
    command_ring: CommandRing<R>,
    event_ring: EventRing<R>,
    waiting_ports: WaitingPort,
    allocator: Rc<RefCell<A>>,
}

impl<Register, Allocator> Controller<Register, Allocator>
where
    Register: XhcRegisters + 'static,
    Allocator: Allocatable,
{
    pub fn new(
        mmio_base: u64,
        mut allocator: Allocator,
        drivers: Vec<Box<dyn ClassDriverOperate>>,
    ) -> Result<Self, ()> {
        let mut registers = Rc::new(RefCell::new(Register::new(mmio_base)));

        // registers.borrow_mut().request_ownership(mmio_base);
        // debug!("OS has owned xHCI Controller.");

        registers.borrow_mut().reset()?;
        registers
            .borrow_mut()
            .write_max_device_slots(DEVICE_SIZE as u8)?;

        let max_scratchpad_len = registers.borrow().max_scratchpad_buf_len();
        let device_manager = make_manager(
            &mut registers,
            DEVICE_SIZE as u8,
            max_scratchpad_len,
            &mut allocator,
            drivers,
        )?;

        let command_ring = make_command_ring(&mut registers, 32, &mut allocator)?;
        let event_ring = make_event_ring(&mut registers, 1, 32, &mut allocator)?;

        debug!("xHCI Controller has been initialized.");

        registers.borrow_mut().run()?;

        debug!("xHCI Controller Started.");

        Ok(Controller {
            mmio_base,
            registers,
            command_ring,
            event_ring,
            device_manager,
            waiting_ports: WaitingPort::default(),
            allocator: Rc::new(RefCell::new(allocator)),
        })
    }

    pub fn reset_port(&mut self) -> Result<(), ()> {
        let connect_ports = self.registers.borrow().connecting_ports();
        if connect_ports.is_empty() {
            return Ok(());
        }

        self.registers
            .borrow_mut()
            .reset_port_at(connect_ports[0])?;

        for port_id in connect_ports.into_iter().skip(1) {
            self.waiting_ports.push(port_id);
        }

        Ok(())
    }

    pub fn start_event_pooling(&mut self) -> ! {
        loop {
            let _ = self.process_event().map(|p| p.unwrap());
        }
    }

    pub fn process_all_event(&mut self) {
        while self.event_ring.has_front() {
            self.process_event();
        }
    }

    pub fn process_event(&mut self) -> Option<Result<(), ()>> {
        if let Some(event_trb) = self.event_ring.read_event_trb() {
            return Some(self.on_event(event_trb));
        }
        None
    }

    fn on_event(&mut self, event_trb: EventTrb) -> Result<(), ()> {
        match event_trb {
            EventTrb::Transfer {
                transfer_event,
                target_event,
            } => self.on_transfer_event(transfer_event, target_event)?,
            EventTrb::PortStatusChange(event) => self.on_port_status_change(event)?,
            EventTrb::CommandCompletion(event) => self.process_completion_event(event)?,
            EventTrb::NotSupport { .. } => {}
        }

        self.event_ring.next_dequeue_pointer()
    }

    fn on_transfer_event(
        &mut self,
        transfer_event: TransferEvent,
        target_event: TargetEvent,
    ) -> Result<(), ()> {
        let slot_id = transfer_event.slot_id();
        let is_init =
            self.device_manager
                .process_transfer_event(slot_id, transfer_event, target_event)?;
        if is_init {
            self.configure_endpoint(slot_id)?;
        }

        Ok(())
    }

    fn process_completion_event(&mut self, event: CommandCompletion) -> Result<(), ()> {
        match TrbRaw::from_addr(event.command_trb_pointer() as u64)
            .template()
            .trb_type()
        {
            9 => self.address_device(event),
            11 => self.init_device(event),
            12 => self.device_manager.configure_endpoint(event.slot_id()),
            _ => Ok(()),
        }?;
        Ok(())
    }

    fn on_port_status_change(&mut self, event: PortStatusChange) -> Result<(), ()> {
        let port_id = event.port_id();
        if self.device_manager.is_addressing_port(port_id) {
            self.enable_slot(port_id)?;
        } else {
            self.waiting_ports.push(port_id);
        }
        Ok(())
    }

    fn configure_endpoint(&mut self, slot_id: u8) -> Result<(), ()> {
        let input_context_addr = self
            .device_manager
            .device_slot_at(slot_id)
            .unwrap()
            .input_context_addr();
        self.command_ring
            .push_configure_endpoint(input_context_addr, slot_id)
    }

    fn init_device(&mut self, event: CommandCompletion) -> Result<(), ()> {
        self.reset_waiting_port()?;

        self.device_manager.initialize_at(event.slot_id())
    }

    fn address_device(&mut self, event: CommandCompletion) -> Result<(), ()> {
        let input_context_addr = self
            .device_manager
            .address_device(event.slot_id(), &self.allocator)?;
        self.command_ring
            .push_address_command(input_context_addr, event.slot_id())
    }

    fn enable_slot(&mut self, port_id: u8) -> Result<(), ()> {
        self.registers
            .borrow_mut()
            .clear_port_reset_change_at(port_id)?;
        self.device_manager.set_addressing_port(port_id);
        self.command_ring.push_enable_slot()
    }

    fn reset_waiting_port(&mut self) -> Result<(), ()> {
        if let Some(port_id) = self.waiting_ports.pop() {
            self.registers.borrow_mut().reset_port_at(port_id)?;
        }
        Ok(())
    }
}

pub static XHC: OnceLock<Mutex<Controller<External, Allocator>>> = OnceLock::new();

pub fn regist_controller(xhc: Controller<External, Allocator>) {
    XHC.get_or_init(|| Mutex::new(xhc));
}
