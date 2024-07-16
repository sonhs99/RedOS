use alloc::vec::Vec;
use core::{mem::size_of, num::NonZeroUsize};
use xhci::{
    accessor, extended_capabilities,
    registers::{self, doorbell::Register, operational::UsbCommandRegister},
    ExtendedCapability, Registers,
};

use super::{
    super::allocator::Allocatable, CapabilityRegisterAccessible, ConfigRegisterAccessible,
    DoorbellRegisterAccessible, InterrupterSetRegisterAccessible, OperationalRegsisterAccessible,
    PortRegisterAccessible, RegisterOperation, UsbCommandRegisterAccessible, XhcRegisters,
};

#[derive(Clone)]
struct Mapper;

impl accessor::Mapper for Mapper {
    unsafe fn map(&mut self, phys_start: usize, bytes: usize) -> NonZeroUsize {
        NonZeroUsize::new_unchecked(phys_start)
    }

    fn unmap(&mut self, virt_start: usize, bytes: usize) {}
}

pub struct External(registers::Registers<Mapper>);

impl External {
    // pub fn new(mmio_base: u64) -> Self {
    //     Self(unsafe { registers::Registers::new(mmio_base as usize, Mapper {}) })
    // }

    pub fn get(&self) -> &Registers<Mapper> {
        &self.0
    }

    pub fn get_mut(&mut self) -> &mut Registers<Mapper> {
        &mut self.0
    }
}

impl RegisterOperation for External {
    fn new(mmio_base: u64) -> Self {
        Self(unsafe { registers::Registers::new(mmio_base as usize, Mapper {}) })
    }

    fn reset(&mut self) -> Result<(), ()> {
        let reg = self.get_mut();
        reg.operational.usbcmd.update_volatile(|usbcmd| {
            usbcmd.clear_run_stop();
        });

        while !reg.operational.usbsts.read_volatile().hc_halted() {}

        reg.operational.usbcmd.update_volatile(|usbcmd| {
            usbcmd.set_host_controller_reset();
        });

        while reg
            .operational
            .usbsts
            .read_volatile()
            .controller_not_ready()
        {}

        Ok(())
    }

    fn run(&mut self) -> Result<(), ()> {
        let reg = self.get_mut();

        reg.operational.usbcmd.update_volatile(|usbcmd| {
            usbcmd.set_interrupter_enable();
        });

        reg.interrupter_register_set
            .interrupter_mut(0)
            .imod
            .update_volatile(|imod| {
                imod.set_interrupt_moderation_interval(100);
            });

        reg.operational.usbcmd.update_volatile(|usbcmd| {
            usbcmd.set_run_stop();
        });

        while reg.operational.usbsts.read_volatile().hc_halted() {}

        Ok(())
    }
}

impl ConfigRegisterAccessible for External {
    fn write_max_device_slots(&mut self, max_slots: u8) -> Result<(), ()> {
        self.0.operational.config.update_volatile(|config| {
            config.set_max_device_slots_enabled(max_slots);
        });
        Ok(())
    }
}

impl CapabilityRegisterAccessible for External {
    fn max_scratchpad_buf_len(&self) -> usize {
        self.0
            .capability
            .hcsparams2
            .read_volatile()
            .max_scratchpad_buffers() as usize
    }

    fn request_ownership(&mut self, mmio_base: u64) {
        let hccparams1 = self.0.capability.hccparams1.read_volatile();
        let mut extcap_list = unsafe {
            extended_capabilities::List::new(mmio_base as usize, hccparams1, Mapper {}).unwrap()
        };
        let legacy = extcap_list.into_iter().find_map(|find| match find {
            Ok(reg) => match reg {
                ExtendedCapability::UsbLegacySupport(ext) => Some(ext),
                _ => None,
            },
            Err(_) => None,
        });
        if let Some(mut dev) = legacy {
            if dev.usblegsup.read_volatile().hc_os_owned_semaphore() {
                return;
            }
            dev.usblegsup.update_volatile(|u| {
                u.set_hc_os_owned_semaphore();
            });
            let mut reg = dev.usblegsup.read_volatile();
            while reg.hc_bios_owned_semaphore() || !reg.hc_os_owned_semaphore() {
                reg = dev.usblegsup.read_volatile();
            }
        }
    }
}

impl OperationalRegsisterAccessible for External {
    fn write_device_context_base_addr(&mut self, base_addr: u64) -> Result<(), ()> {
        self.0.operational.dcbaap.update_volatile(|dcbaap| {
            dcbaap.set(base_addr);
        });
        Ok(())
    }
}

impl DoorbellRegisterAccessible for External {
    fn notify_at(&mut self, index: usize, target: u8, stream_id: u16) -> Result<(), ()> {
        self.0.doorbell.update_volatile_at(index, |doorbell| {
            doorbell.set_doorbell_target(target);
            doorbell.set_doorbell_stream_id(stream_id);
        });
        Ok(())
    }
}

impl PortRegisterAccessible for External {
    fn reset_port_at(&mut self, port_id: u8) -> Result<(), ()> {
        self.0
            .port_register_set
            .update_volatile_at(port_id as usize, |reg| {
                reg.portsc.set_port_reset();
            });
        while self
            .0
            .port_register_set
            .read_volatile_at(port_id as usize)
            .portsc
            .port_reset()
        {}
        Ok(())
    }

    fn read_port_speed_at(&self, port_id: u8) -> Result<u8, ()> {
        Ok(self
            .0
            .port_register_set
            .read_volatile_at(port_index(port_id))
            .portsc
            .port_speed())
    }

    fn read_port_reset_change_status(&self, port_id: u8) -> Result<bool, ()> {
        Ok(self
            .0
            .port_register_set
            .read_volatile_at(port_index(port_id))
            .portsc
            .port_reset_change())
    }

    fn clear_port_reset_change_at(&mut self, port_id: u8) -> Result<(), ()> {
        self.0
            .port_register_set
            .update_volatile_at(port_index(port_id), |reg| {
                reg.portsc.set_0_port_reset_change();
            });
        Ok(())
    }

    fn reset_all(&mut self) {
        let ports = self
            .0
            .port_register_set
            .into_iter()
            .enumerate()
            .filter(|(_, port)| port.portsc.current_connect_status())
            .map(|(idx, _)| idx)
            .collect::<Vec<usize>>();

        self.0
            .port_register_set
            .update_volatile_at(ports[0], |reg| {
                reg.portsc.set_port_reset();
            });

        while self
            .0
            .port_register_set
            .read_volatile_at(ports[0])
            .portsc
            .port_reset()
        {}

        ports.into_iter().for_each(|port_id| {
            self.0.port_register_set.update_volatile_at(port_id, |reg| {
                reg.portsc.set_port_reset();
            });
            while self
                .0
                .port_register_set
                .read_volatile_at(port_id)
                .portsc
                .port_reset()
            {}
        });
    }

    fn connecting_ports(&self) -> Vec<u8> {
        self.0
            .port_register_set
            .into_iter()
            .enumerate()
            .filter(|(_, port)| port.portsc.current_connect_status())
            .map(|(idx, _)| idx as u8)
            .collect()
    }
}

fn port_index(port_id: u8) -> usize {
    (port_id - 1) as usize
}

impl UsbCommandRegisterAccessible for External {
    fn write_command_ring_addr(&mut self, command_ring_addr: u64) -> Result<(), ()> {
        self.0.operational.crcr.update_volatile(|crcr| {
            crcr.set_ring_cycle_state();
            crcr.set_command_ring_pointer(command_ring_addr);
        });
        Ok(())
    }
}

impl InterrupterSetRegisterAccessible for External {
    fn clear_interrupt_pending_at(&mut self, index: usize) {
        self.0
            .interrupter_register_set
            .interrupter_mut(index)
            .iman
            .update_volatile(|iman| {
                iman.clear_interrupt_pending();
            })
    }

    fn clear_event_handler_busy_at(&mut self, index: usize) {
        self.0
            .interrupter_register_set
            .interrupter_mut(index)
            .erdp
            .update_volatile(|erdp| {
                erdp.clear_event_handler_busy();
            })
    }

    fn set_counter_at(&mut self, index: usize, count: u16) {
        self.0
            .interrupter_register_set
            .interrupter_mut(index)
            .imod
            .update_volatile(|imod| {
                imod.set_interrupt_moderation_counter(count);
            });
    }

    fn write_event_ring_dequeue_pointer_at(
        &mut self,
        index: usize,
        event_ring_segment_addr: u64,
    ) -> Result<(), ()> {
        self.0
            .interrupter_register_set
            .interrupter_mut(index)
            .erdp
            .update_volatile(|erdp| {
                erdp.set_event_ring_dequeue_pointer(event_ring_segment_addr);
            });
        self.clear_event_handler_busy_at(index);
        Ok(())
    }

    fn write_event_ring_segment_table_pointer_at(
        &mut self,
        index: usize,
        event_ring_segment_table_addr: u64,
    ) -> Result<(), ()> {
        self.0
            .interrupter_register_set
            .interrupter_mut(index)
            .erstba
            .update_volatile(|erstba| erstba.set(event_ring_segment_table_addr));
        Ok(())
    }

    fn write_interrupter_enable_at(&mut self, index: usize, is_enable: bool) -> Result<(), ()> {
        self.0
            .interrupter_register_set
            .interrupter_mut(index)
            .iman
            .update_volatile(|iman| {
                if is_enable {
                    iman.set_interrupt_enable();
                } else {
                    iman.clear_interrupt_enable();
                }
            });
        Ok(())
    }

    fn write_interrupter_pending_at(&mut self, index: usize, is_pending: bool) -> Result<(), ()> {
        self.0
            .interrupter_register_set
            .interrupter_mut(index)
            .iman
            .update_volatile(|iman| {
                if is_pending {
                    iman.set_0_interrupt_pending();
                } else {
                    iman.clear_interrupt_pending();
                }
            });
        Ok(())
    }

    fn read_dequeue_pointer_addr_at(&mut self, index: usize) -> u64 {
        self.0
            .interrupter_register_set
            .interrupter(index)
            .erdp
            .read_volatile()
            .event_ring_dequeue_pointer()
    }

    fn write_event_ring_segment_table_size(&mut self, index: usize, size: u16) -> Result<(), ()> {
        self.0
            .interrupter_register_set
            .interrupter_mut(index)
            .erstsz
            .update_volatile(|erstsz| erstsz.set(size));
        Ok(())
    }
}

impl XhcRegisters for External {}
