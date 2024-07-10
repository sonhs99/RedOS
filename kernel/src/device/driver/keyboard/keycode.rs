#[repr(u8)]
#[derive(Clone, Copy)]
pub enum KeySpecial {
    None = 0x00,
    Enter = b'\n',
    Tab = b'\t',
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

#[derive(Clone, Copy)]
pub enum Key {
    Ascii(u8),
    Special(KeySpecial),
}

impl Key {
    pub fn is_alpha(&self) -> bool {
        match self {
            Self::Ascii(code) => {
                (*code >= b'a' && *code <= b'z') || (*code >= b'A' && *code <= b'Z')
            }
            Self::Special(_) => false,
        }
    }

    pub fn is_num(&self) -> bool {
        match self {
            Self::Ascii(code) => *code >= b'0' && *code <= b'9',
            Self::Special(_) => false,
        }
    }
}

pub struct KeyMappingEntry(Key, Key);

impl KeyMappingEntry {
    pub const fn new(normal: Key, combined: Key) -> Self {
        Self(normal, combined)
    }
    pub fn normal(&self) -> Key {
        self.0
    }

    pub fn combined(&self) -> Key {
        self.1
    }
}
