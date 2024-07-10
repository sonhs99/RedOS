mod external;

use alloc::vec::Vec;
use core::{mem::size_of, num::NonZeroUsize};
use xhci::{
    accessor, extended_capabilities,
    registers::{self, doorbell::Register},
    ExtendedCapability, Registers,
};

use super::allocator::Allocatable;
pub use external::External;

pub trait RegisterOperation {
    fn new(mmio_base: u64) -> Self;
    fn reset(&mut self) -> Result<(), ()>;
    fn run(&mut self) -> Result<(), ()>;
}

pub trait ConfigRegisterAccessible {
    fn write_max_device_slots(&mut self, max_slots: u8) -> Result<(), ()>;
}

pub trait CapablitiyRegisterAccessible {
    fn max_scratchpad_buf_len(&self) -> usize;
    fn request_ownership(&mut self, mmio_base: u64);
}

pub trait OperationalRegsisterAccessible {
    fn write_device_context_base_addr(&mut self, base_addr: u64) -> Result<(), ()>;

    fn setup_device_context_array(
        &mut self,
        device_slots: u8,
        scratchpad_buffer_len: usize,
        allocator: &mut impl Allocatable,
    ) -> Result<&'static mut [u64], ()> {
        let context_array = allocator.alloc_array::<u64>((device_slots + 1) as usize, 64, 4096)?;
        if scratchpad_buffer_len > 0 {
            let scratchpad_buffer_array =
                allocator.alloc_array::<*mut u64>(scratchpad_buffer_len, 4096, 4096)?;
            for cell in scratchpad_buffer_array.iter_mut() {
                *cell = allocator.alloc(size_of::<xhci::context::Device32Byte>(), 64, 0)?;
            }
            context_array[0] = scratchpad_buffer_array.as_ptr() as u64;
        }
        self.write_device_context_base_addr(context_array.as_ptr() as u64);
        Ok(context_array)
    }
}

pub trait DoorbellRegisterAccessible {
    fn notify_at(&mut self, index: usize, target: u8, stream_id: u16) -> Result<(), ()>;
}

pub trait PortRegisterAccessible {
    fn reset_port_at(&mut self, port_id: u8) -> Result<(), ()>;
    fn read_port_speed_at(&self, port_id: u8) -> Result<u8, ()>;
    fn read_port_reset_change_status(&self, port_id: u8) -> Result<bool, ()>;
    fn clear_port_reset_change_at(&mut self, port_id: u8) -> Result<(), ()>;
    fn reset_all(&mut self);
    fn connecting_ports(&self) -> Vec<u8>;
}

pub trait UsbCommandRegisterAccessible {
    fn write_command_ring_addr(&mut self, command_ring_addr: u64) -> Result<(), ()>;
}

pub trait InterrupterSetRegisterAccessible {
    fn clear_interrupt_pending_at(&mut self, index: usize);
    fn clear_event_handler_busy_at(&mut self, index: usize);
    fn set_counter_at(&mut self, index: usize, count: u16);
    fn write_event_ring_dequeue_pointer_at(
        &mut self,
        index: usize,
        event_ring_segment_addr: u64,
    ) -> Result<(), ()>;
    fn write_event_ring_segment_table_pointer_at(
        &mut self,
        index: usize,
        event_ring_segment_table_addr: u64,
    ) -> Result<(), ()>;
    fn write_interrupter_enable_at(&mut self, index: usize, is_enable: bool) -> Result<(), ()>;
    fn write_interrupter_pending_at(&mut self, index: usize, is_pending: bool) -> Result<(), ()>;
    fn read_dequeue_pointer_addr_at(&mut self, index: usize) -> u64;
    fn write_event_ring_segment_table_size(&mut self, index: usize, size: u16) -> Result<(), ()>;

    fn update_dequeue_pointer_at(
        &mut self,
        index: usize,
        dequeue_pointer_addr: u64,
    ) -> Result<(), ()> {
        self.write_interrupter_pending_at(index, true)?;
        self.write_event_ring_dequeue_pointer_at(index, dequeue_pointer_addr)?;
        Ok(())
    }
}

pub trait XhcRegisters:
    RegisterOperation
    + ConfigRegisterAccessible
    + CapablitiyRegisterAccessible
    + DoorbellRegisterAccessible
    + PortRegisterAccessible
    + OperationalRegsisterAccessible
    + UsbCommandRegisterAccessible
    + InterrupterSetRegisterAccessible
{
}
