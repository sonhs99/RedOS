use alloc::rc::Rc;
use log::debug;

use super::{
    keycode::{Key, KeyMappingEntry, KeySpecial, Keycode},
    manager::KeyboardManager,
};

const KEY_MAPPINGTABLEMAXCOUNT: usize = 89;

const KEY_SKIPCOUNTFORPAUSE: u8 = 2;

pub const KEY_FLAG_UP: u8 = 0x00;
pub const KEY_FLAG_DOWN: u8 = 0x01;
pub const KEY_FLAG_EXTENDKEY: u8 = 0x02;

const KEY_MAPPING_TABLE: [KeyMappingEntry; KEY_MAPPINGTABLEMAXCOUNT] = [
    /*  0   */
    KeyMappingEntry::new(
        Key::Special(KeySpecial::None),
        Key::Special(KeySpecial::None),
    ),
    /*  1   */
    KeyMappingEntry::new(Key::Special(KeySpecial::Esc), Key::Special(KeySpecial::Esc)),
    /*  2   */ KeyMappingEntry::new(Key::Ascii(b'1'), Key::Ascii(b'!')),
    /*  3   */ KeyMappingEntry::new(Key::Ascii(b'2'), Key::Ascii(b'@')),
    /*  4   */ KeyMappingEntry::new(Key::Ascii(b'3'), Key::Ascii(b'#')),
    /*  5   */ KeyMappingEntry::new(Key::Ascii(b'4'), Key::Ascii(b'$')),
    /*  6   */ KeyMappingEntry::new(Key::Ascii(b'5'), Key::Ascii(b'%')),
    /*  7   */ KeyMappingEntry::new(Key::Ascii(b'6'), Key::Ascii(b'^')),
    /*  8   */ KeyMappingEntry::new(Key::Ascii(b'7'), Key::Ascii(b'&')),
    /*  9   */ KeyMappingEntry::new(Key::Ascii(b'8'), Key::Ascii(b'*')),
    /*  10  */ KeyMappingEntry::new(Key::Ascii(b'9'), Key::Ascii(b'(')),
    /*  11  */ KeyMappingEntry::new(Key::Ascii(b'0'), Key::Ascii(b')')),
    /*  12  */ KeyMappingEntry::new(Key::Ascii(b'-'), Key::Ascii(b'_')),
    /*  13  */ KeyMappingEntry::new(Key::Ascii(b'='), Key::Ascii(b'+')),
    /*  14  */
    KeyMappingEntry::new(
        Key::Special(KeySpecial::Backspace),
        Key::Special(KeySpecial::Backspace),
    ),
    /*  15  */
    KeyMappingEntry::new(Key::Special(KeySpecial::Tab), Key::Special(KeySpecial::Tab)),
    /*  16  */ KeyMappingEntry::new(Key::Ascii(b'q'), Key::Ascii(b'Q')),
    /*  17  */ KeyMappingEntry::new(Key::Ascii(b'w'), Key::Ascii(b'W')),
    /*  18  */ KeyMappingEntry::new(Key::Ascii(b'e'), Key::Ascii(b'E')),
    /*  19  */ KeyMappingEntry::new(Key::Ascii(b'r'), Key::Ascii(b'R')),
    /*  20  */ KeyMappingEntry::new(Key::Ascii(b't'), Key::Ascii(b'T')),
    /*  21  */ KeyMappingEntry::new(Key::Ascii(b'y'), Key::Ascii(b'Y')),
    /*  22  */ KeyMappingEntry::new(Key::Ascii(b'u'), Key::Ascii(b'U')),
    /*  23  */ KeyMappingEntry::new(Key::Ascii(b'i'), Key::Ascii(b'I')),
    /*  24  */ KeyMappingEntry::new(Key::Ascii(b'o'), Key::Ascii(b'O')),
    /*  25  */ KeyMappingEntry::new(Key::Ascii(b'p'), Key::Ascii(b'P')),
    /*  26  */ KeyMappingEntry::new(Key::Ascii(b'['), Key::Ascii(b'{')),
    /*  27  */ KeyMappingEntry::new(Key::Ascii(b']'), Key::Ascii(b'}')),
    /*  28  */
    KeyMappingEntry::new(
        Key::Special(KeySpecial::Enter),
        Key::Special(KeySpecial::Enter),
    ),
    /*  29  */
    KeyMappingEntry::new(
        Key::Special(KeySpecial::Ctrl),
        Key::Special(KeySpecial::Ctrl),
    ),
    /*  30  */ KeyMappingEntry::new(Key::Ascii(b'a'), Key::Ascii(b'A')),
    /*  31  */ KeyMappingEntry::new(Key::Ascii(b's'), Key::Ascii(b'S')),
    /*  32  */ KeyMappingEntry::new(Key::Ascii(b'd'), Key::Ascii(b'D')),
    /*  33  */ KeyMappingEntry::new(Key::Ascii(b'f'), Key::Ascii(b'F')),
    /*  34  */ KeyMappingEntry::new(Key::Ascii(b'g'), Key::Ascii(b'G')),
    /*  35  */ KeyMappingEntry::new(Key::Ascii(b'h'), Key::Ascii(b'H')),
    /*  36  */ KeyMappingEntry::new(Key::Ascii(b'j'), Key::Ascii(b'J')),
    /*  37  */ KeyMappingEntry::new(Key::Ascii(b'k'), Key::Ascii(b'K')),
    /*  38  */ KeyMappingEntry::new(Key::Ascii(b'l'), Key::Ascii(b'L')),
    /*  39  */ KeyMappingEntry::new(Key::Ascii(b';'), Key::Ascii(b':')),
    /*  40  */ KeyMappingEntry::new(Key::Ascii(b'\''), Key::Ascii(b'\"')),
    /*  41  */ KeyMappingEntry::new(Key::Ascii(b'`'), Key::Ascii(b'~')),
    /*  42  */
    KeyMappingEntry::new(
        Key::Special(KeySpecial::Lshift),
        Key::Special(KeySpecial::Lshift),
    ),
    /*  43  */ KeyMappingEntry::new(Key::Ascii(b'\\'), Key::Ascii(b'|')),
    /*  44  */ KeyMappingEntry::new(Key::Ascii(b'z'), Key::Ascii(b'Z')),
    /*  45  */ KeyMappingEntry::new(Key::Ascii(b'x'), Key::Ascii(b'X')),
    /*  46  */ KeyMappingEntry::new(Key::Ascii(b'c'), Key::Ascii(b'C')),
    /*  47  */ KeyMappingEntry::new(Key::Ascii(b'v'), Key::Ascii(b'V')),
    /*  48  */ KeyMappingEntry::new(Key::Ascii(b'b'), Key::Ascii(b'B')),
    /*  49  */ KeyMappingEntry::new(Key::Ascii(b'n'), Key::Ascii(b'N')),
    /*  50  */ KeyMappingEntry::new(Key::Ascii(b'm'), Key::Ascii(b'M')),
    /*  51  */ KeyMappingEntry::new(Key::Ascii(b','), Key::Ascii(b'<')),
    /*  52  */ KeyMappingEntry::new(Key::Ascii(b'.'), Key::Ascii(b'>')),
    /*  53  */ KeyMappingEntry::new(Key::Ascii(b'/'), Key::Ascii(b'?')),
    /*  54  */
    KeyMappingEntry::new(
        Key::Special(KeySpecial::Rshift),
        Key::Special(KeySpecial::Rshift),
    ),
    /*  55  */ KeyMappingEntry::new(Key::Ascii(b'*'), Key::Ascii(b'*')),
    /*  56  */
    KeyMappingEntry::new(
        Key::Special(KeySpecial::Lalt),
        Key::Special(KeySpecial::Lalt),
    ),
    /*  57  */ KeyMappingEntry::new(Key::Ascii(b' '), Key::Ascii(b' ')),
    /*  58  */
    KeyMappingEntry::new(
        Key::Special(KeySpecial::CapsLock),
        Key::Special(KeySpecial::CapsLock),
    ),
    /*  59  */
    KeyMappingEntry::new(Key::Special(KeySpecial::F1), Key::Special(KeySpecial::F1)),
    /*  60  */
    KeyMappingEntry::new(Key::Special(KeySpecial::F2), Key::Special(KeySpecial::F2)),
    /*  61  */
    KeyMappingEntry::new(Key::Special(KeySpecial::F3), Key::Special(KeySpecial::F3)),
    /*  62  */
    KeyMappingEntry::new(Key::Special(KeySpecial::F4), Key::Special(KeySpecial::F4)),
    /*  63  */
    KeyMappingEntry::new(Key::Special(KeySpecial::F5), Key::Special(KeySpecial::F5)),
    /*  64  */
    KeyMappingEntry::new(Key::Special(KeySpecial::F6), Key::Special(KeySpecial::F6)),
    /*  65  */
    KeyMappingEntry::new(Key::Special(KeySpecial::F7), Key::Special(KeySpecial::F7)),
    /*  66  */
    KeyMappingEntry::new(Key::Special(KeySpecial::F8), Key::Special(KeySpecial::F8)),
    /*  67  */
    KeyMappingEntry::new(Key::Special(KeySpecial::F9), Key::Special(KeySpecial::F9)),
    /*  68  */
    KeyMappingEntry::new(Key::Special(KeySpecial::F10), Key::Special(KeySpecial::F10)),
    /*  69  */
    KeyMappingEntry::new(
        Key::Special(KeySpecial::NumLock),
        Key::Special(KeySpecial::NumLock),
    ),
    /*  70  */
    KeyMappingEntry::new(
        Key::Special(KeySpecial::ScrollLock),
        Key::Special(KeySpecial::ScrollLock),
    ),
    /*  71  */ KeyMappingEntry::new(Key::Special(KeySpecial::Home), Key::Ascii(b'7')),
    /*  72  */ KeyMappingEntry::new(Key::Special(KeySpecial::Up), Key::Ascii(b'8')),
    /*  73  */ KeyMappingEntry::new(Key::Special(KeySpecial::PageUp), Key::Ascii(b'9')),
    /*  74  */ KeyMappingEntry::new(Key::Ascii(b'-'), Key::Ascii(b'-')),
    /*  75  */ KeyMappingEntry::new(Key::Special(KeySpecial::Left), Key::Ascii(b'4')),
    /*  76  */ KeyMappingEntry::new(Key::Special(KeySpecial::Center), Key::Ascii(b'5')),
    /*  77  */ KeyMappingEntry::new(Key::Special(KeySpecial::Right), Key::Ascii(b'6')),
    /*  78  */ KeyMappingEntry::new(Key::Ascii(b'+'), Key::Ascii(b'+')),
    /*  79  */ KeyMappingEntry::new(Key::Special(KeySpecial::End), Key::Ascii(b'1')),
    /*  80  */ KeyMappingEntry::new(Key::Special(KeySpecial::Down), Key::Ascii(b'2')),
    /*  81  */
    KeyMappingEntry::new(Key::Special(KeySpecial::PageDown), Key::Ascii(b'3')),
    /*  82  */ KeyMappingEntry::new(Key::Special(KeySpecial::Insert), Key::Ascii(b'0')),
    /*  83  */ KeyMappingEntry::new(Key::Special(KeySpecial::Delete), Key::Ascii(b'.')),
    /*  84  */
    KeyMappingEntry::new(
        Key::Special(KeySpecial::None),
        Key::Special(KeySpecial::None),
    ),
    /*  85  */
    KeyMappingEntry::new(
        Key::Special(KeySpecial::None),
        Key::Special(KeySpecial::None),
    ),
    /*  86  */
    KeyMappingEntry::new(
        Key::Special(KeySpecial::None),
        Key::Special(KeySpecial::None),
    ),
    /*  87  */
    KeyMappingEntry::new(Key::Special(KeySpecial::F11), Key::Special(KeySpecial::F11)),
    /*  88  */
    KeyMappingEntry::new(Key::Special(KeySpecial::F12), Key::Special(KeySpecial::F12)),
];

#[derive(Clone, Copy)]
pub struct ScanCode(u8);

impl ScanCode {
    pub const fn down_scan_code(&self) -> u8 {
        self.0 & 0x7F
    }

    pub const fn is_down(&self) -> bool {
        (self.0 & 0x80) != 0
    }
}

impl Keycode for ScanCode {
    fn is_alpha(&self) -> bool {
        let down_scan_code = self.down_scan_code();
        let resolved_code = KEY_MAPPING_TABLE[down_scan_code as usize].normal();
        resolved_code.is_alpha()
    }

    fn is_common_num(&self) -> bool {
        ((2 <= self.0) && (53 >= self.0)) && !self.is_alpha()
    }

    fn is_num_pad(&self) -> bool {
        (71 <= self.0) && (83 >= self.0)
    }
}

trait KeyboardSubscriber {
    fn subscribe(&self, keycode: Key, pressed: bool);
}

impl<F> KeyboardSubscriber for F
where
    F: Fn(Key, bool),
{
    fn subscribe(&self, keycode: Key, pressed: bool) {
        self(keycode, pressed)
    }
}

pub struct PS2KeyboardDriver {
    subsrcribe: Rc<dyn KeyboardSubscriber>,
    manager: KeyboardManager,
    skip_count_for_pause: usize,
    shift_pressed: bool,
    extended_code: bool,
}

impl PS2KeyboardDriver {
    pub fn new<F>(subsrcribe: F) -> Self
    where
        F: Fn(Key, bool) + 'static,
    {
        Self {
            subsrcribe: Rc::new(subsrcribe),
            manager: KeyboardManager::new(),
            skip_count_for_pause: 0,
            shift_pressed: false,
            extended_code: false,
        }
    }

    pub fn scan_code_to_ascii(&mut self, scan_code: ScanCode) -> Result<(Key, bool), ()> {
        let down_scan_code = scan_code.down_scan_code();
        if self.skip_count_for_pause > 0 {
            self.skip_count_for_pause -= 1;
            return Err(());
        }

        if down_scan_code == 0xE1 {
            self.skip_count_for_pause = 3;
            return Ok((Key::Special(KeySpecial::Pause), true));
        } else if down_scan_code == 0xE0 {
            self.extended_code = true;
            return Err(());
        }

        // if (down_scan_code as usize) > KEY_MAPPING_TABLE.len() {
        //     return Err(());
        // }

        let key = if self.manager.is_combined_code(scan_code, self.shift_pressed) {
            KEY_MAPPING_TABLE[down_scan_code as usize].combined()
        } else {
            KEY_MAPPING_TABLE[down_scan_code as usize].normal()
        };

        if self.extended_code {
            self.extended_code = false;
        }

        if scan_code.is_down() {
            self.manager.update_key_status(&key);
        }

        if let Key::Special(special_key) = key {
            if special_key == KeySpecial::Lshift || special_key == KeySpecial::Rshift {
                self.shift_pressed = scan_code.is_down();
            }
        }
        Ok((key, scan_code.is_down()))
    }

    pub fn on_data_received(&mut self, key_code: u8) {
        if let Ok((key, pressed)) = self.scan_code_to_ascii(ScanCode(key_code)) {
            self.subsrcribe.subscribe(key, pressed);
        }
    }
}
