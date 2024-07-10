use core::arch::asm;

const PAGE_SIZE_4K: usize = 4096;
const PAGE_SIZE_2M: usize = PAGE_SIZE_4K * 512;
const PAGE_SIZE_1G: usize = PAGE_SIZE_2M * 512;
const PAGE_DIRECTORY_COUNT: usize = 64;

pub const UEFI_PAGE_SIZE: usize = 4096;

#[repr(align(4096))]
#[derive(Clone, Copy)]
struct PageTable([u64; 512]);

impl PageTable {
    pub const fn new() -> Self {
        Self([0u64; 512])
    }
}

static mut PML4_TABLE: PageTable = PageTable::new();
static mut PDP_TABLE: PageTable = PageTable::new();
static mut PD_TABLE: [PageTable; PAGE_DIRECTORY_COUNT] = [PageTable::new(); PAGE_DIRECTORY_COUNT];

unsafe fn init_page_unsafe() {
    PML4_TABLE.0[0] = PDP_TABLE.0.as_ptr() as u64 | 0x03;
    for pdp_idx in 0..PAGE_DIRECTORY_COUNT {
        PDP_TABLE.0[pdp_idx] = PD_TABLE[pdp_idx].0.as_ptr() as u64 | 0x03;
        for pd_idx in 0..512 {
            PD_TABLE[pdp_idx].0[pd_idx] =
                (pdp_idx * PAGE_SIZE_1G + pd_idx * PAGE_SIZE_2M | 0x83) as u64;
        }
    }
    asm!(
        "mov cr3, {}",
        in(reg) PML4_TABLE.0.as_ptr() as u64
    );
}

pub fn init_page() {
    unsafe { init_page_unsafe() };
}
