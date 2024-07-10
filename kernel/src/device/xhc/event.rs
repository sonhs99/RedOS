use xhci::ring::trb::transfer::{DataStage, Normal, SetupStage, StatusStage};

use super::trb::{TrbRaw, TrbTemplate};

#[derive(Debug)]
pub enum TargetEvent {
    Normal(Normal),
    Setup(SetupStage),
    Data(DataStage),
    Status(StatusStage),
}

impl TargetEvent {
    pub fn new(target_addr: u64) -> Option<Self> {
        let raw_data = TrbRaw::from_addr(target_addr);
        match raw_data.template().trb_type() {
            1 => Some(TargetEvent::Normal(
                Normal::try_from(raw_data.as_array()).ok()?,
            )),
            2 => Some(TargetEvent::Setup(
                SetupStage::try_from(raw_data.as_array()).ok()?,
            )),
            3 => Some(TargetEvent::Data(
                DataStage::try_from(raw_data.as_array()).ok()?,
            )),
            4 => Some(TargetEvent::Status(
                StatusStage::try_from(raw_data.as_array()).ok()?,
            )),
            _ => None,
        }
    }

    pub fn data_stage(self) -> Result<DataStage, ()> {
        if let TargetEvent::Data(stage) = self {
            Ok(stage)
        } else {
            Err(())
        }
    }

    pub fn status_stage(self) -> Result<StatusStage, ()> {
        if let TargetEvent::Status(stage) = self {
            Ok(stage)
        } else {
            Err(())
        }
    }

    pub fn normal(self) -> Result<Normal, ()> {
        if let TargetEvent::Normal(stage) = self {
            Ok(stage)
        } else {
            Err(())
        }
    }
}
