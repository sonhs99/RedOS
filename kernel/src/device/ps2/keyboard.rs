use crate::sync::{Mutex, OnceLock};

use super::super::Port;

const KEY_MAPPINGTABLEMAXCOUNT: usize = 89;

const KEY_SKIPCOUNTFORPAUSE: u8 = 2;

pub const KEY_FLAG_UP: u8 = 0x00;
pub const KEY_FLAG_DOWN: u8 = 0x01;
pub const KEY_FLAG_EXTENDKEY: u8 = 0x02;

const KEY_MAPPING_TABLE: [KeyMappingEntry; KEY_MAPPINGTABLEMAXCOUNT] = [
    /*  0   */ KeyMappingEntry(KeySpecial::None as u8, KeySpecial::None as u8),
    /*  1   */ KeyMappingEntry(KeySpecial::Esc as u8, KeySpecial::Esc as u8),
    /*  2   */ KeyMappingEntry('1' as u8, '!' as u8),
    /*  3   */ KeyMappingEntry('2' as u8, '@' as u8),
    /*  4   */ KeyMappingEntry('3' as u8, '#' as u8),
    /*  5   */ KeyMappingEntry('4' as u8, '$' as u8),
    /*  6   */ KeyMappingEntry('5' as u8, '%' as u8),
    /*  7   */ KeyMappingEntry('6' as u8, '^' as u8),
    /*  8   */ KeyMappingEntry('7' as u8, '&' as u8),
    /*  9   */ KeyMappingEntry('8' as u8, '*' as u8),
    /*  10  */ KeyMappingEntry('9' as u8, '(' as u8),
    /*  11  */ KeyMappingEntry('0' as u8, ')' as u8),
    /*  12  */ KeyMappingEntry('-' as u8, '_' as u8),
    /*  13  */ KeyMappingEntry('=' as u8, '+' as u8),
    /*  14  */ KeyMappingEntry(KeySpecial::Backspace as u8, KeySpecial::Backspace as u8),
    /*  15  */ KeyMappingEntry(KeySpecial::Tab as u8, KeySpecial::Tab as u8),
    /*  16  */ KeyMappingEntry('q' as u8, 'Q' as u8),
    /*  17  */ KeyMappingEntry('w' as u8, 'W' as u8),
    /*  18  */ KeyMappingEntry('e' as u8, 'E' as u8),
    /*  19  */ KeyMappingEntry('r' as u8, 'R' as u8),
    /*  20  */ KeyMappingEntry('t' as u8, 'T' as u8),
    /*  21  */ KeyMappingEntry('y' as u8, 'Y' as u8),
    /*  22  */ KeyMappingEntry('u' as u8, 'U' as u8),
    /*  23  */ KeyMappingEntry('i' as u8, 'I' as u8),
    /*  24  */ KeyMappingEntry('o' as u8, 'O' as u8),
    /*  25  */ KeyMappingEntry('p' as u8, 'P' as u8),
    /*  26  */ KeyMappingEntry('[' as u8, '{' as u8),
    /*  27  */ KeyMappingEntry(']' as u8, '}' as u8),
    /*  28  */ KeyMappingEntry('\n' as u8, '\n' as u8),
    /*  29  */ KeyMappingEntry(KeySpecial::Ctrl as u8, KeySpecial::Ctrl as u8),
    /*  30  */ KeyMappingEntry('a' as u8, 'A' as u8),
    /*  31  */ KeyMappingEntry('s' as u8, 'S' as u8),
    /*  32  */ KeyMappingEntry('d' as u8, 'D' as u8),
    /*  33  */ KeyMappingEntry('f' as u8, 'F' as u8),
    /*  34  */ KeyMappingEntry('g' as u8, 'G' as u8),
    /*  35  */ KeyMappingEntry('h' as u8, 'H' as u8),
    /*  36  */ KeyMappingEntry('j' as u8, 'J' as u8),
    /*  37  */ KeyMappingEntry('k' as u8, 'K' as u8),
    /*  38  */ KeyMappingEntry('l' as u8, 'L' as u8),
    /*  39  */ KeyMappingEntry(';' as u8, ':' as u8),
    /*  40  */ KeyMappingEntry('\'' as u8, '\"' as u8),
    /*  41  */ KeyMappingEntry('`' as u8, '~' as u8),
    /*  42  */ KeyMappingEntry(KeySpecial::Lshift as u8, KeySpecial::Lshift as u8),
    /*  43  */ KeyMappingEntry('\\' as u8, '|' as u8),
    /*  44  */ KeyMappingEntry('z' as u8, 'Z' as u8),
    /*  45  */ KeyMappingEntry('x' as u8, 'X' as u8),
    /*  46  */ KeyMappingEntry('c' as u8, 'C' as u8),
    /*  47  */ KeyMappingEntry('v' as u8, 'V' as u8),
    /*  48  */ KeyMappingEntry('b' as u8, 'B' as u8),
    /*  49  */ KeyMappingEntry('n' as u8, 'N' as u8),
    /*  50  */ KeyMappingEntry('m' as u8, 'M' as u8),
    /*  51  */ KeyMappingEntry(',' as u8, '<' as u8),
    /*  52  */ KeyMappingEntry('.' as u8, '>' as u8),
    /*  53  */ KeyMappingEntry('/' as u8, '?' as u8),
    /*  54  */ KeyMappingEntry(KeySpecial::Rshift as u8, KeySpecial::Rshift as u8),
    /*  55  */ KeyMappingEntry('*' as u8, '*' as u8),
    /*  56  */ KeyMappingEntry(KeySpecial::Lalt as u8, KeySpecial::Lalt as u8),
    /*  57  */ KeyMappingEntry(' ' as u8, ' ' as u8),
    /*  58  */ KeyMappingEntry(KeySpecial::CapsLock as u8, KeySpecial::CapsLock as u8),
    /*  59  */ KeyMappingEntry(KeySpecial::F1 as u8, KeySpecial::F1 as u8),
    /*  60  */ KeyMappingEntry(KeySpecial::F2 as u8, KeySpecial::F2 as u8),
    /*  61  */ KeyMappingEntry(KeySpecial::F3 as u8, KeySpecial::F3 as u8),
    /*  62  */ KeyMappingEntry(KeySpecial::F4 as u8, KeySpecial::F4 as u8),
    /*  63  */ KeyMappingEntry(KeySpecial::F5 as u8, KeySpecial::F5 as u8),
    /*  64  */ KeyMappingEntry(KeySpecial::F6 as u8, KeySpecial::F6 as u8),
    /*  65  */ KeyMappingEntry(KeySpecial::F7 as u8, KeySpecial::F7 as u8),
    /*  66  */ KeyMappingEntry(KeySpecial::F8 as u8, KeySpecial::F8 as u8),
    /*  67  */ KeyMappingEntry(KeySpecial::F9 as u8, KeySpecial::F9 as u8),
    /*  68  */ KeyMappingEntry(KeySpecial::F10 as u8, KeySpecial::F10 as u8),
    /*  69  */ KeyMappingEntry(KeySpecial::NumLock as u8, KeySpecial::NumLock as u8),
    /*  70  */ KeyMappingEntry(KeySpecial::ScrollLock as u8, KeySpecial::ScrollLock as u8),
    /*  71  */ KeyMappingEntry(KeySpecial::Home as u8, '7' as u8),
    /*  72  */ KeyMappingEntry(KeySpecial::Up as u8, '8' as u8),
    /*  73  */ KeyMappingEntry(KeySpecial::PageUp as u8, '9' as u8),
    /*  74  */ KeyMappingEntry('-' as u8, '-' as u8),
    /*  75  */ KeyMappingEntry(KeySpecial::Left as u8, '4' as u8),
    /*  76  */ KeyMappingEntry(KeySpecial::Center as u8, '5' as u8),
    /*  77  */ KeyMappingEntry(KeySpecial::Right as u8, '6' as u8),
    /*  78  */ KeyMappingEntry('+' as u8, '+' as u8),
    /*  79  */ KeyMappingEntry(KeySpecial::End as u8, '1' as u8),
    /*  80  */ KeyMappingEntry(KeySpecial::Down as u8, '2' as u8),
    /*  81  */ KeyMappingEntry(KeySpecial::PageDown as u8, '3' as u8),
    /*  82  */ KeyMappingEntry(KeySpecial::Insert as u8, '0' as u8),
    /*  83  */ KeyMappingEntry(KeySpecial::Delete as u8, '.' as u8),
    /*  84  */ KeyMappingEntry(KeySpecial::None as u8, KeySpecial::None as u8),
    /*  85  */ KeyMappingEntry(KeySpecial::None as u8, KeySpecial::None as u8),
    /*  86  */ KeyMappingEntry(KeySpecial::None as u8, KeySpecial::None as u8),
    /*  87  */ KeyMappingEntry(KeySpecial::F11 as u8, KeySpecial::F11 as u8),
    /*  88  */ KeyMappingEntry(KeySpecial::F12 as u8, KeySpecial::F12 as u8),
];

pub static KEYBOARD_MANAGER: OnceLock<Mutex<KeyboardManager>> = OnceLock::new();
pub static KEYBOARD: OnceLock<Mutex<Keyboard>> = OnceLock::new();

pub struct Keyboard {
    control: Port,
    state: Port,
    input_buffer: Port,
    output_buffer: Port,
}
pub enum KeySpecial {
    None = 0x00,
    Enter = '\n' as isize,
    Tab = '\t' as isize,
    Esc = 0x1B,
    Backspace = 0x08,
    Ctrl = 0x81,
    Lshift = 0x82,
    Rshift = 0x83,
    PrintScreen = 0x84,
    Lalt = 0x85,
    CapsLock = 0x86,
    F1 = 0x87,
    F2 = 0x88,
    F3 = 0x89,
    F4 = 0x8A,
    F5 = 0x8B,
    F6 = 0x8C,
    F7 = 0x8D,
    F8 = 0x8E,
    F9 = 0x8F,
    F10 = 0x90,
    NumLock = 0x91,
    ScrollLock = 0x92,
    Home = 0x93,
    Up = 0x94,
    PageUp = 0x95,
    Left = 0x96,
    Center = 0x97,
    Right = 0x98,
    End = 0x99,
    Down = 0x9A,
    PageDown = 0x9B,
    Insert = 0x9C,
    Delete = 0x9D,
    F11 = 0x9E,
    F12 = 0x9F,
    Pause = 0xA0,
}

pub struct KeyMappingEntry(u8, u8);

pub struct KeyboardManager {
    shift_down: bool,
    capslock: bool,
    numlock: bool,
    scrolllock: bool,

    extended_code: bool,
    skip_count_for_pause: i32,
}

#[derive(Clone, Copy)]
pub struct ScanCode(u8);

impl Keyboard {
    pub const fn new(control: u16, state: u16, input_buffer: u16, output_buffer: u16) -> Self {
        Keyboard {
            control: Port::new(control),
            state: Port::new(state),
            input_buffer: Port::new(input_buffer),
            output_buffer: Port::new(output_buffer),
        }
    }

    pub fn is_input_buffer_full(&self) -> bool {
        (self.state.in8() & 0x02) != 0
    }

    pub fn is_output_buffer_full(&self) -> bool {
        (self.state.in8() & 0x01) != 0
    }

    pub fn activate(&self) -> Result<(), ()> {
        self.control.out8(0xAE);
        for _ in 0..0xFFFF {
            if !self.is_input_buffer_full() {
                break;
            }
        }

        self.input_buffer.out8(0xF4);
        for _ in 0..100 {
            for _ in 0..0xFFFF {
                if self.is_output_buffer_full() {
                    break;
                }
            }
            if self.output_buffer.in8() == 0xFA {
                return Ok(());
            }
        }
        Err(())
    }

    pub fn get_scan_code(&self) -> ScanCode {
        while !self.is_output_buffer_full() {}
        ScanCode(self.output_buffer.in8())
    }

    pub fn enable_a20_gate(&self) {
        self.control.out8(0xD0);
        for _ in 0..0xFFFF {
            if self.is_output_buffer_full() {
                break;
            }
        }
        let data = self.output_buffer.in8();
        let data = data | 0x01;
        for _ in 0..0xFFFF {
            if !self.is_input_buffer_full() {
                break;
            }
        }
        self.control.out8(0xD1);
        self.input_buffer.out8(data);
    }

    pub fn change_led(&self, capslock: bool, numlock: bool, scrolllock: bool) {
        for _ in 0..0xFFFF {
            if !self.is_input_buffer_full() {
                break;
            }
        }
        self.output_buffer.out8(0xED);
        for _ in 0..0xFFFF {
            if !self.is_input_buffer_full() {
                break;
            }
        }
        for _ in 0..100 {
            for _ in 0..0xFFFF {
                if self.is_output_buffer_full() {
                    break;
                }
            }
            if self.output_buffer.in8() == 0xFA {
                break;
            }
        }
        self.output_buffer
            .out8((capslock as u8) << 2 | (numlock as u8) << 1 | scrolllock as u8);
        for _ in 0..0xFFFF {
            if !self.is_input_buffer_full() {
                break;
            }
        }
        for _ in 0..100 {
            for _ in 0..0xFFFF {
                if self.is_output_buffer_full() {
                    break;
                }
            }
            if self.output_buffer.in8() == 0xFA {
                break;
            }
        }
    }
}

impl KeyboardManager {
    pub const fn new() -> Self {
        KeyboardManager {
            shift_down: false,
            capslock: false,
            numlock: false,
            scrolllock: false,
            extended_code: false,
            skip_count_for_pause: 0,
        }
    }

    pub const fn is_combined_code(&self, scan_code: ScanCode) -> bool {
        if scan_code.is_alpha() {
            self.shift_down ^ self.capslock
        } else if scan_code.is_num_or_sym() {
            self.shift_down
        } else if scan_code.is_num_pad() && !self.extended_code {
            self.numlock
        } else {
            false
        }
    }

    pub fn update_key_status(&mut self, scan_code: ScanCode) {
        let down_scan_code = scan_code.down_scan_code();
        let down = scan_code.is_down();
        let led_status_change = if (down_scan_code == 42) || (down_scan_code == 54) {
            self.shift_down = scan_code.is_down();
            false
        } else if (down_scan_code == 58) && down {
            self.capslock ^= true;
            true
        } else if (down_scan_code == 69) && down {
            self.numlock ^= true;
            true
        } else if (down_scan_code == 70) && down {
            self.scrolllock ^= true;
            true
        } else {
            false
        };

        if led_status_change {
            change_keyboard_led(self.capslock, self.numlock, self.scrolllock);
        }
    }
}

impl KeyMappingEntry {
    pub const fn combined(&self) -> u8 {
        self.1
    }

    pub const fn normal(&self) -> u8 {
        self.0
    }
}

impl ScanCode {
    pub const fn down_scan_code(&self) -> u8 {
        self.0 & 0x7F
    }

    pub const fn is_alpha(&self) -> bool {
        let resolved_code = KEY_MAPPING_TABLE[self.0 as usize].normal();
        (('a' as u8) <= resolved_code) && (('z' as u8) >= resolved_code)
    }

    pub const fn is_num_or_sym(&self) -> bool {
        ((2 <= self.0) && (53 >= self.0)) && !self.is_alpha()
    }

    pub const fn is_num_pad(&self) -> bool {
        (71 <= self.0) && (83 >= self.0)
    }

    pub const fn is_down(&self) -> bool {
        (self.0 & 0x80) != 0
    }
}

pub fn scan_code_to_ascii(scan_code: ScanCode) -> Result<(u8, u8), ()> {
    let down_scan_code = scan_code.down_scan_code();
    let mut manager = KEYBOARD_MANAGER.lock();
    if manager.skip_count_for_pause > 0 {
        manager.skip_count_for_pause -= 1;
        return Err(());
    }

    if down_scan_code == 0xE1 {
        manager.skip_count_for_pause = 3;
        return Ok((KeySpecial::Pause as u8, KEY_FLAG_DOWN));
    } else if down_scan_code == 0xE0 {
        manager.extended_code = true;
        return Err(());
    }

    let ascii = if manager.is_combined_code(scan_code) {
        KEY_MAPPING_TABLE[down_scan_code as usize].combined()
    } else {
        KEY_MAPPING_TABLE[down_scan_code as usize].normal()
    };

    let flag = if manager.extended_code {
        manager.extended_code = false;
        KEY_FLAG_EXTENDKEY
    } else {
        0
    };

    let flag = if scan_code.is_down() {
        flag | KEY_FLAG_DOWN
    } else {
        flag
    };

    manager.update_key_status(scan_code);
    Ok((ascii, flag))
}

fn change_keyboard_led(capslock: bool, numlock: bool, scrolllock: bool) {
    KEYBOARD.lock().change_led(capslock, numlock, scrolllock);
}
