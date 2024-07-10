use xhci::context::{EndpointHandler, EndpointType};

use super::{descriptor::structure::EndpointDescriptor, device::DeviceContextIndex};

#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub struct EndpointId(usize);

impl EndpointId {
    pub fn from_addr(addr: usize) -> Self {
        Self(addr)
    }

    pub fn from_endpoint_num(ep_num: usize, dir_in: bool) -> Self {
        Self((ep_num << 1) | dir_in as usize)
    }
    pub fn is_control_in(&self) -> bool {
        self.0 & 0b1 == 1
    }
    pub fn value(&self) -> usize {
        self.0
    }
}

impl Default for EndpointId {
    fn default() -> Self {
        EndpointId::from_endpoint_num(0, true)
    }
}

#[derive(Debug, Clone)]
pub struct EndpointConfig {
    endpoint_id: EndpointId,
    endpoint_type: EndpointType,
    max_packet_size: u16,
    interval: u8,
}

impl EndpointConfig {
    pub fn new(endpoint: &EndpointDescriptor) -> Self {
        let ep_num = endpoint.endpoint_address().number() as usize;
        let dir_in = endpoint.endpoint_address().dir_in();
        let transfer_type = endpoint.attributes().transfer_type();

        Self {
            endpoint_id: EndpointId::from_endpoint_num(ep_num, dir_in),
            endpoint_type: to_endpoint_type(transfer_type, dir_in),
            max_packet_size: endpoint.max_packet_size(),
            interval: endpoint.interval(),
        }
    }

    pub fn endpoint_id(&self) -> EndpointId {
        self.endpoint_id
    }

    pub fn endpoint_type(&self) -> EndpointType {
        self.endpoint_type
    }

    pub fn max_packet_size(&self) -> u16 {
        self.max_packet_size
    }

    pub fn interval(&self) -> u8 {
        self.interval
    }

    pub fn device_context_index(&self) -> DeviceContextIndex {
        DeviceContextIndex::from_endpoint(self.endpoint_id)
    }

    pub fn write_endpoint_context(
        &self,
        tr_buff_addr: u64,
        endpoint_ctx: &mut dyn EndpointHandler,
    ) {
        endpoint_ctx.set_endpoint_type(EndpointType::InterruptIn);
        endpoint_ctx.set_tr_dequeue_pointer(tr_buff_addr);
        endpoint_ctx.set_max_packet_size(self.max_packet_size);
        endpoint_ctx.set_interval(self.interval - 1);
        endpoint_ctx.set_average_trb_length(1);
        endpoint_ctx.set_error_count(3);
        endpoint_ctx.set_mult(0);
        endpoint_ctx.set_max_primary_streams(0);
        endpoint_ctx.set_dequeue_cycle_state();
    }
}

fn to_endpoint_type(v: u8, dir_in: bool) -> EndpointType {
    match v {
        0 => EndpointType::NotValid,
        1 => EndpointType::IsochOut,
        2 => EndpointType::BulkOut,
        3 => {
            if dir_in {
                EndpointType::InterruptIn
            } else {
                EndpointType::InterruptOut
            }
        }
        4 => EndpointType::Control,
        5 => EndpointType::IsochIn,
        6 => EndpointType::BulkIn,
        _ => EndpointType::NotValid,
    }
}
