static mut RANDOM_VALUE: u64 = 0;

pub fn seed(sd: u64) {
    unsafe { RANDOM_VALUE = sd };
}

pub fn random() -> u64 {
    unsafe {
        RANDOM_VALUE = RANDOM_VALUE.wrapping_mul(412153);
        RANDOM_VALUE = (RANDOM_VALUE.wrapping_add(5571031)) >> 16;
        RANDOM_VALUE
    }
}

pub fn abs(value: isize) -> usize {
    if value > 0 {
        value as usize
    } else {
        (-value) as usize
    }
}
