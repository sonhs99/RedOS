use core::{cell::RefCell, mem::size_of, ptr::slice_from_raw_parts_mut};

use alloc::rc::Rc;
use log::debug;
use xhci::ring::trb::{
    command::{AddressDevice, ConfigureEndpoint, EnableSlot, Noop, ResetEndpoint},
    event::{CommandCompletion, PortStatusChange, TransferEvent},
    transfer::{DataStage, Normal, StatusStage},
    Link,
};

use super::{
    allocator::Allocatable,
    event::TargetEvent,
    register::{
        DoorbellRegisterAccessible, InterrupterSetRegisterAccessible, UsbCommandRegisterAccessible,
    },
    trb::TrbRaw,
};

const fn trb_size() -> usize {
    size_of::<u128>()
}

fn trb_buffer_from_address(address: *mut u128) -> &'static mut [u32; 4] {
    unsafe {
        let raw_data = address.cast::<u32>();
        &mut *(slice_from_raw_parts_mut(raw_data, 4) as *mut [u32; 4])
    }
}

#[derive(Debug, Clone, Copy)]
pub struct TransferRing {
    ring_base_addr: u64,
    ring_ptr_addr: u64,
    ring_end_addr: u64,
    ring_size: usize,
    cycle_bit: bool,
}

impl TransferRing {
    pub fn new(ring_base_addr: u64, ring_size: usize, cycle_bit: bool) -> Self {
        Self {
            ring_base_addr,
            ring_ptr_addr: ring_base_addr,
            ring_end_addr: ring_base_addr + (trb_size() * (ring_size - 1)) as u64,
            ring_size,
            cycle_bit,
        }
    }

    pub fn new_with_alloc(
        ring_size: usize,
        cycle_bit: bool,
        allocator: &mut impl Allocatable,
    ) -> Result<Self, ()> {
        let base_ptr = unsafe {
            match allocator.allocate(trb_size() * ring_size, 64, 4096) {
                Some(ptr) => Ok(ptr),
                None => Err(()),
            }?
        };
        Ok(Self::new(base_ptr as u64, ring_size, cycle_bit))
    }

    pub fn push(&mut self, trb: [u32; 4]) -> Result<(), ()> {
        self.write(trb)?;
        self.ring_ptr_addr += trb_size() as u64;
        if self.is_end_event_address(self.ring_ptr_addr) {
            self.rollback()?
        }
        Ok(())
    }

    pub fn read(&self) -> Option<TrbRaw> {
        self.read_trb(self.ring_ptr_addr)
    }

    pub fn read_trb(&self, trb_addr: u64) -> Option<TrbRaw> {
        if trb_addr == 0 {
            return None;
        }
        Some(TrbRaw::from_addr(trb_addr))
    }

    fn rollback(&mut self) -> Result<(), ()> {
        let mut link = Link::new();
        link.set_toggle_cycle();
        link.set_ring_segment_pointer(self.ring_base_addr);

        self.write(link.into_raw())?;

        self.ring_ptr_addr = self.ring_base_addr;
        self.toggle_cycle_bit();
        Ok(())
    }

    fn write(&mut self, src_buff: [u32; 4]) -> Result<(), ()> {
        let buff = trb_buffer_from_address(self.ring_ptr_addr as *mut u128);
        buff[0] = src_buff[0];
        buff[1] = src_buff[1];
        buff[2] = src_buff[2];
        buff[3] = (src_buff[3] & !0b1) | self.cycle_bit as u32;
        Ok(())
    }

    pub fn base_address(&self) -> u64 {
        self.ring_base_addr
    }

    pub fn toggle_cycle_bit(&mut self) {
        self.cycle_bit = !self.cycle_bit;
    }

    pub fn cycle_bit(&self) -> bool {
        self.cycle_bit
    }

    pub fn is_end_event_address(&self, addr: u64) -> bool {
        addr >= self.ring_end_addr + trb_size() as u64
    }
}

pub struct CommandRing<D> {
    transfer_ring: TransferRing,
    doorbell: Rc<RefCell<D>>,
}

impl<D> CommandRing<D>
where
    D: DoorbellRegisterAccessible,
{
    pub fn new(ring_ptr_addr: u64, ring_size: usize, doorbell: &Rc<RefCell<D>>) -> Self {
        Self {
            transfer_ring: TransferRing::new(ring_ptr_addr, ring_size, true),
            doorbell: doorbell.clone(),
        }
    }

    pub fn push_no_op(&mut self) -> Result<(), ()> {
        self.transfer_ring.push(Noop::new().into_raw())?;
        self.notify()
    }

    pub fn push_reset_endpoint(&mut self, slot_id: u8, endpoint_id: u8) -> Result<(), ()> {
        let mut reset_endpoint = ResetEndpoint::new();
        reset_endpoint.set_endpoint_id(endpoint_id);
        reset_endpoint.set_slot_id(slot_id);

        self.transfer_ring.push(reset_endpoint.into_raw())?;
        self.notify()
    }

    pub fn push_configure_endpoint(
        &mut self,
        input_context_addr: u64,
        slot_id: u8,
    ) -> Result<(), ()> {
        let mut configure_endpoint_trb = ConfigureEndpoint::new();
        configure_endpoint_trb.set_slot_id(slot_id);
        configure_endpoint_trb.set_input_context_pointer(input_context_addr);

        self.transfer_ring.push(configure_endpoint_trb.into_raw())?;
        self.notify()
    }

    pub fn push_address_command(&mut self, input_context_addr: u64, slot_id: u8) -> Result<(), ()> {
        let mut address_command = AddressDevice::new();
        address_command.set_input_context_pointer(input_context_addr);
        address_command.set_slot_id(slot_id);

        self.transfer_ring.push(address_command.into_raw())?;
        self.notify()
    }

    pub fn push_enable_slot(&mut self) -> Result<(), ()> {
        self.transfer_ring.push(EnableSlot::new().into_raw())?;
        self.notify()
    }

    fn notify(&mut self) -> Result<(), ()> {
        self.doorbell.borrow_mut().notify_at(0, 0, 0)
    }
}

pub(crate) fn make_command_ring<T>(
    registers: &mut Rc<RefCell<T>>,
    command_ring_size: usize,
    allocator: &mut impl Allocatable,
) -> Result<CommandRing<T>, ()>
where
    T: DoorbellRegisterAccessible + UsbCommandRegisterAccessible,
{
    let command_ring_addr = unsafe {
        match allocator.allocate(trb_size() * command_ring_size, 64, 4096) {
            Some(ptr) => Ok(ptr),
            None => Err(()),
        }? as u64
    };
    let command_ring = CommandRing::new(
        command_ring_addr & !0b00111111,
        command_ring_size,
        registers,
    );
    registers
        .borrow_mut()
        .write_command_ring_addr(command_ring_addr)?;
    Ok(command_ring)
}

#[derive(Debug)]
pub enum EventTrb {
    Transfer {
        transfer_event: TransferEvent,
        target_event: TargetEvent,
    },
    PortStatusChange(PortStatusChange),
    CommandCompletion(CommandCompletion),
    NotSupport {
        trb_type: u8,
    },
}

impl EventTrb {
    pub fn new(trb: TrbRaw, cycle_bit: bool) -> Option<Self> {
        let trb_buff: [u32; 4] = trb.as_array();
        if ((trb.raw() >> 96) & 0b01 == 1) != cycle_bit {
            // if !cycle_bit {
            //     debug!("trb={}, bit={}", (trb.raw() >> 96) & 0b01, cycle_bit as u8);
            // }
            return None;
        }
        // debug!("trb={}, bit={}", (trb.raw() >> 96) & 0b01, cycle_bit as u8);
        let event_trb = match trb.template().trb_type() {
            32 => EventTrb::Transfer {
                transfer_event: TransferEvent::try_from(trb_buff).ok()?,
                target_event: read_target_trb(TransferEvent::try_from(trb_buff).ok()?)?,
            },
            33 => EventTrb::CommandCompletion(CommandCompletion::try_from(trb_buff).ok()?),
            34 => EventTrb::PortStatusChange(PortStatusChange::try_from(trb_buff).ok()?),
            _ => EventTrb::NotSupport {
                trb_type: trb.template().trb_type(),
            },
        };
        Some(event_trb)
    }

    pub fn circle_bit(&self) -> Option<bool> {
        match self {
            EventTrb::Transfer {
                transfer_event,
                target_event,
            } => Some(transfer_event.cycle_bit()),
            EventTrb::PortStatusChange(e) => Some(e.cycle_bit()),
            EventTrb::CommandCompletion(e) => Some(e.cycle_bit()),
            EventTrb::NotSupport { trb_type } => None,
        }
    }
}

fn read_target_trb(transfer_event: TransferEvent) -> Option<TargetEvent> {
    let raw_data = unsafe { *(transfer_event.trb_pointer() as *const u128) };
    let trb = TrbRaw::new_unchecked(raw_data);

    match trb.template().trb_type() {
        1 => Some(TargetEvent::Normal(Normal::try_from(trb.as_array()).ok()?)),
        3 => Some(TargetEvent::Data(DataStage::try_from(trb.as_array()).ok()?)),
        4 => Some(TargetEvent::Status(
            StatusStage::try_from(trb.as_array()).ok()?,
        )),
        _ => None,
    }
}

pub struct EventRing<I> {
    transfer_ring: TransferRing,
    segment_base_addr: u64,
    interrupter_set: Rc<RefCell<I>>,
}

impl<I> EventRing<I>
where
    I: InterrupterSetRegisterAccessible,
{
    pub fn new(segment_base_addr: u64, ring_size: usize, interrupter_set: &Rc<RefCell<I>>) -> Self {
        Self {
            transfer_ring: TransferRing::new(
                interrupter_set.borrow_mut().read_dequeue_pointer_addr_at(0),
                ring_size,
                true,
            ),
            segment_base_addr,
            interrupter_set: interrupter_set.clone(),
        }
    }

    pub fn has_front(&self) -> bool {
        if let Some(circle_bit) = self.read_event_trb().and_then(|trb| trb.circle_bit()) {
            circle_bit == self.transfer_ring.cycle_bit()
        } else {
            false
        }
    }

    pub fn read(&self) -> Option<EventTrb> {
        let event_ring_dequeue_pointer_addr = self.read_dequeue_pointer_addr() + trb_size() as u64;
        let trb_raw =
            TrbRaw::new_unchecked(unsafe { *(event_ring_dequeue_pointer_addr as *mut u128) });
        EventTrb::new(trb_raw, self.transfer_ring.cycle_bit())
    }

    pub fn read_event_trb(&self) -> Option<EventTrb> {
        let event_ring_dequeue_pointer_addr = self.read_dequeue_pointer_addr();
        let trb_raw =
            TrbRaw::new_unchecked(unsafe { *(event_ring_dequeue_pointer_addr as *mut u128) });
        EventTrb::new(trb_raw, self.transfer_ring.cycle_bit())
    }

    pub fn next_dequeue_pointer(&mut self) -> Result<(), ()> {
        let dequeue_pointer_addr = self.read_dequeue_pointer_addr();
        let next_addr = dequeue_pointer_addr + trb_size() as u64;
        if self.transfer_ring.is_end_event_address(next_addr) {
            self.transfer_ring.toggle_cycle_bit();
            self.write_dequeue_pointer(self.segment_base_addr)
        } else {
            self.write_dequeue_pointer(next_addr)
        }
    }

    fn read_dequeue_pointer_addr(&self) -> u64 {
        self.interrupter_set
            .borrow_mut()
            .read_dequeue_pointer_addr_at(0)
    }

    fn write_dequeue_pointer(&mut self, addr: u64) -> Result<(), ()> {
        self.interrupter_set
            .borrow_mut()
            .update_dequeue_pointer_at(0, addr)
    }
}

#[repr(C, align(64))]
pub struct EventRingSegmentTableEntry {
    pub ring_segment_base_address: u64,
    pub ring_segment_size: u16,
    _reserved1: u16,
    _reserved2: u32,
}

pub fn make_event_ring<I>(
    registers: &mut Rc<RefCell<I>>,
    event_ring_segment_table_size: u16,
    event_ring_segment_size: usize,
    allocator: &mut impl Allocatable,
) -> Result<EventRing<I>, ()>
where
    I: InterrupterSetRegisterAccessible,
{
    let event_ring_segment_table_addr = unsafe {
        match allocator.allocate(
            trb_size() * event_ring_segment_table_size as usize,
            64,
            4096,
        ) {
            Some(ptr) => Ok(ptr),
            None => Err(()),
        }? as u64
    };
    let event_ring_segment_addr = unsafe {
        match allocator.allocate(trb_size() * event_ring_segment_size, 64, 4096) {
            Some(ptr) => Ok(ptr),
            None => Err(()),
        }? as u64
    };
    registers
        .borrow_mut()
        .write_event_ring_segment_table_size(0, event_ring_segment_table_size)?;
    registers
        .borrow_mut()
        .write_event_ring_dequeue_pointer_at(0, event_ring_segment_addr)?;

    let segment_table =
        unsafe { &mut *(event_ring_segment_table_addr as *mut EventRingSegmentTableEntry) };
    segment_table.ring_segment_base_address = event_ring_segment_addr & !0b0011_1111;
    segment_table.ring_segment_size = event_ring_segment_size as u16;

    registers
        .borrow_mut()
        .write_event_ring_segment_table_pointer_at(0, event_ring_segment_table_addr);

    registers
        .borrow_mut()
        .write_interrupter_enable_at(0, true)?;
    let event_ring = EventRing::new(event_ring_segment_addr, event_ring_segment_size, registers);
    Ok(event_ring)
}
