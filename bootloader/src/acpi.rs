use core::intrinsics::compare_bytes;

#[repr(C, packed)]
pub struct RSDP {
    signature: [u8; 8],
    checksum: u8,
    oem_id: [u8; 6],
    revision: u8,
    rsdt_address: u32,
    length: u32,
    pub xsdt_address: u64,
    extend_checksum: u8,
    _reserved: [u8; 3],
}

impl RSDP {
    pub fn is_valid(&self) -> bool {
        let ptr = (self as *const RSDP).cast::<u8>();
        if unsafe { compare_bytes(self.signature.as_ptr(), b"RSD PTR ".as_ptr(), 8) != 0 } {
            false
        } else if self.revision != 2 {
            false
        } else if sum(ptr, 20) != 0 {
            false
        } else if sum(ptr, 36) != 0 {
            false
        } else {
            true
        }
    }
}

fn sum(ptr: *const u8, size: usize) -> u8 {
    let mut count = 0u8;
    for idx in 0..size {
        unsafe {
            count = count.wrapping_add(*ptr.add(idx));
        }
    }
    count
}
