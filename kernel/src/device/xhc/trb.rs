use modular_bitfield::bitfield;
use modular_bitfield::prelude::B6;
#[bitfield(bits = 128)]
#[derive(Clone, Copy)]
pub struct TrbTemplate {
    pub parameter: u64,
    pub status: u32,
    pub cycle_bit: bool,
    pub evaluate_next_trb: bool,
    #[allow(non_snake_case)]
    _reserve1: u8,
    pub trb_type: B6,
    pub control: u16,
}

impl TrbTemplate {
    pub fn from_addr(addr: u64) -> Self {
        unsafe { *(addr as *const Self) }
    }

    pub fn as_array(&self) -> [u32; 4] {
        let value = unsafe { *(self as *const TrbTemplate as *const u128) };
        let mask = |shift: u128| (value >> (shift * 32)) as u32;
        [mask(0), mask(1), mask(2), mask(3)]
    }
}

pub struct TrbRaw(u128);

impl TrbRaw {
    pub fn new_unchecked(raw: u128) -> Self {
        Self(raw)
    }

    pub fn from_addr(addr: u64) -> Self {
        let raw = unsafe { *(addr as *const u128) };
        Self::new_unchecked(raw)
    }

    pub fn template(&self) -> TrbTemplate {
        unsafe { *((&self.0 as *const u128).cast::<TrbTemplate>()) }
    }

    pub fn raw(&self) -> u128 {
        self.0
    }

    pub fn as_array(&self) -> [u32; 4] {
        let value = self.0;
        let mask = |shift: u128| (value >> (shift * 32)) as u32;
        [mask(0), mask(1), mask(2), mask(3)]
    }
}

impl From<[u32; 4]> for TrbRaw {
    fn from(value: [u32; 4]) -> Self {
        let mask = |raw: u32, shift: u128| (raw as u128) << (shift * 32);
        let raw = mask(value[0], 0) | mask(value[1], 1) | mask(value[2], 2) | mask(value[3], 3);
        Self::new_unchecked(raw)
    }
}
