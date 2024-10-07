use crate::sync::OnceLock;

static CPU_COUNT: OnceLock<usize> = OnceLock::new();
static LOCAL_APIC_REGISTER_BASE: OnceLock<u32> = OnceLock::new();

pub fn init(cpu_count: usize, lapic_base: u32) {
    CPU_COUNT.get_or_init(|| cpu_count);
    LOCAL_APIC_REGISTER_BASE.get_or_init(|| lapic_base);
}

pub fn get_cpu_count() -> usize {
    *CPU_COUNT.get().unwrap()
}

pub fn lapic_register_base() -> u32 {
    LOCAL_APIC_REGISTER_BASE
        .get()
        .cloned()
        .unwrap_or(0xFEE0_0000u32)
}
