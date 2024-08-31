use crate::sync::OnceLock;

static CPU_COUNT: OnceLock<usize> = OnceLock::new();

pub fn init(cpu_count: usize) {
    CPU_COUNT.get_or_init(|| cpu_count);
}

pub fn get_cpu_count() -> usize {
    *CPU_COUNT.get().unwrap()
}
