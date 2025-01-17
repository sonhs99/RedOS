use super::{
    keycode::{Key, KeyMappingEntry, KeySpecial, Keycode},
    manager::KeyboardManager,
};
use crate::device::xhc::driver::{ClassDriverOperate, DriverType};
use alloc::{rc::Rc, vec::Vec};
use log::{debug, error};

const COMBINE_KEY_LSHIFT: u8 = 0b0000_0010;
const COMBINE_KEY_RSHIFT: u8 = 0b0010_0000;

const KEY_MAPPING_TABLE: [KeyMappingEntry; 103] = [
    /*  000  */
    KeyMappingEntry::new(
        Key::Special(KeySpecial::None),
        Key::Special(KeySpecial::None),
    ),
    /*  001  */
    KeyMappingEntry::new(
        Key::Special(KeySpecial::None),
        Key::Special(KeySpecial::None),
    ),
    /*  002  */
    KeyMappingEntry::new(
        Key::Special(KeySpecial::None),
        Key::Special(KeySpecial::None),
    ),
    /*  003  */
    KeyMappingEntry::new(
        Key::Special(KeySpecial::None),
        Key::Special(KeySpecial::None),
    ),
    /*  004  */ KeyMappingEntry::new(Key::Ascii(b'a'), Key::Ascii(b'A')),
    /*  005  */ KeyMappingEntry::new(Key::Ascii(b'b'), Key::Ascii(b'B')),
    /*  006  */ KeyMappingEntry::new(Key::Ascii(b'c'), Key::Ascii(b'C')),
    /*  007  */ KeyMappingEntry::new(Key::Ascii(b'd'), Key::Ascii(b'D')),
    /*  008  */ KeyMappingEntry::new(Key::Ascii(b'e'), Key::Ascii(b'E')),
    /*  009  */ KeyMappingEntry::new(Key::Ascii(b'f'), Key::Ascii(b'F')),
    /*  010  */ KeyMappingEntry::new(Key::Ascii(b'g'), Key::Ascii(b'G')),
    /*  011  */ KeyMappingEntry::new(Key::Ascii(b'h'), Key::Ascii(b'H')),
    /*  012  */ KeyMappingEntry::new(Key::Ascii(b'i'), Key::Ascii(b'I')),
    /*  013  */ KeyMappingEntry::new(Key::Ascii(b'j'), Key::Ascii(b'J')),
    /*  014  */ KeyMappingEntry::new(Key::Ascii(b'k'), Key::Ascii(b'K')),
    /*  015  */ KeyMappingEntry::new(Key::Ascii(b'l'), Key::Ascii(b'L')),
    /*  016  */ KeyMappingEntry::new(Key::Ascii(b'm'), Key::Ascii(b'M')),
    /*  017  */ KeyMappingEntry::new(Key::Ascii(b'n'), Key::Ascii(b'N')),
    /*  018  */ KeyMappingEntry::new(Key::Ascii(b'o'), Key::Ascii(b'O')),
    /*  019  */ KeyMappingEntry::new(Key::Ascii(b'p'), Key::Ascii(b'P')),
    /*  020  */ KeyMappingEntry::new(Key::Ascii(b'q'), Key::Ascii(b'Q')),
    /*  021  */ KeyMappingEntry::new(Key::Ascii(b'r'), Key::Ascii(b'R')),
    /*  022  */ KeyMappingEntry::new(Key::Ascii(b's'), Key::Ascii(b'S')),
    /*  023  */ KeyMappingEntry::new(Key::Ascii(b't'), Key::Ascii(b'T')),
    /*  024  */ KeyMappingEntry::new(Key::Ascii(b'u'), Key::Ascii(b'U')),
    /*  025  */ KeyMappingEntry::new(Key::Ascii(b'v'), Key::Ascii(b'V')),
    /*  026  */ KeyMappingEntry::new(Key::Ascii(b'w'), Key::Ascii(b'W')),
    /*  027  */ KeyMappingEntry::new(Key::Ascii(b'x'), Key::Ascii(b'X')),
    /*  028  */ KeyMappingEntry::new(Key::Ascii(b'y'), Key::Ascii(b'Y')),
    /*  029  */ KeyMappingEntry::new(Key::Ascii(b'z'), Key::Ascii(b'Z')),
    /*  030  */ KeyMappingEntry::new(Key::Ascii(b'1'), Key::Ascii(b'!')),
    /*  031  */ KeyMappingEntry::new(Key::Ascii(b'2'), Key::Ascii(b'@')),
    /*  032  */ KeyMappingEntry::new(Key::Ascii(b'3'), Key::Ascii(b'#')),
    /*  033  */ KeyMappingEntry::new(Key::Ascii(b'4'), Key::Ascii(b'$')),
    /*  034  */ KeyMappingEntry::new(Key::Ascii(b'5'), Key::Ascii(b'%')),
    /*  035  */ KeyMappingEntry::new(Key::Ascii(b'6'), Key::Ascii(b'^')),
    /*  036  */ KeyMappingEntry::new(Key::Ascii(b'7'), Key::Ascii(b'&')),
    /*  037  */ KeyMappingEntry::new(Key::Ascii(b'8'), Key::Ascii(b'*')),
    /*  038  */ KeyMappingEntry::new(Key::Ascii(b'9'), Key::Ascii(b'(')),
    /*  039  */ KeyMappingEntry::new(Key::Ascii(b'0'), Key::Ascii(b')')),
    /*  040  */
    KeyMappingEntry::new(
        Key::Special(KeySpecial::Enter),
        Key::Special(KeySpecial::Enter),
    ),
    /*  041  */
    KeyMappingEntry::new(Key::Special(KeySpecial::Esc), Key::Special(KeySpecial::Esc)),
    /*  042  */
    KeyMappingEntry::new(
        Key::Special(KeySpecial::Backspace),
        Key::Special(KeySpecial::Backspace),
    ),
    /*  043  */
    KeyMappingEntry::new(Key::Special(KeySpecial::Tab), Key::Special(KeySpecial::Tab)),
    /*  044  */ KeyMappingEntry::new(Key::Ascii(b' '), Key::Ascii(b' ')),
    /*  045  */ KeyMappingEntry::new(Key::Ascii(b'-'), Key::Ascii(b'_')),
    /*  046  */ KeyMappingEntry::new(Key::Ascii(b'='), Key::Ascii(b'+')),
    /*  047  */ KeyMappingEntry::new(Key::Ascii(b'['), Key::Ascii(b'{')),
    /*  048  */ KeyMappingEntry::new(Key::Ascii(b']'), Key::Ascii(b'}')),
    /*  049  */ KeyMappingEntry::new(Key::Ascii(b'\\'), Key::Ascii(b'|')),
    /*  050  */ KeyMappingEntry::new(Key::Ascii(b'\\'), Key::Ascii(b'|')),
    /*  051  */ KeyMappingEntry::new(Key::Ascii(b';'), Key::Ascii(b':')),
    /*  052  */ KeyMappingEntry::new(Key::Ascii(b'\''), Key::Ascii(b'\"')),
    /*  053  */ KeyMappingEntry::new(Key::Ascii(b'`'), Key::Ascii(b'~')),
    /*  054  */ KeyMappingEntry::new(Key::Ascii(b','), Key::Ascii(b'<')),
    /*  055  */ KeyMappingEntry::new(Key::Ascii(b'.'), Key::Ascii(b'>')),
    /*  056  */ KeyMappingEntry::new(Key::Ascii(b'/'), Key::Ascii(b'?')),
    /*  057  */
    KeyMappingEntry::new(
        Key::Special(KeySpecial::CapsLock),
        Key::Special(KeySpecial::CapsLock),
    ),
    /*  058  */
    KeyMappingEntry::new(Key::Special(KeySpecial::F1), Key::Special(KeySpecial::F1)),
    /*  059  */
    KeyMappingEntry::new(Key::Special(KeySpecial::F2), Key::Special(KeySpecial::F2)),
    /*  060  */
    KeyMappingEntry::new(Key::Special(KeySpecial::F3), Key::Special(KeySpecial::F3)),
    /*  061  */
    KeyMappingEntry::new(Key::Special(KeySpecial::F4), Key::Special(KeySpecial::F4)),
    /*  062  */
    KeyMappingEntry::new(Key::Special(KeySpecial::F5), Key::Special(KeySpecial::F5)),
    /*  063  */
    KeyMappingEntry::new(Key::Special(KeySpecial::F6), Key::Special(KeySpecial::F6)),
    /*  064  */
    KeyMappingEntry::new(Key::Special(KeySpecial::F7), Key::Special(KeySpecial::F7)),
    /*  065  */
    KeyMappingEntry::new(Key::Special(KeySpecial::F8), Key::Special(KeySpecial::F8)),
    /*  066  */
    KeyMappingEntry::new(Key::Special(KeySpecial::F9), Key::Special(KeySpecial::F9)),
    /*  067  */
    KeyMappingEntry::new(Key::Special(KeySpecial::F10), Key::Special(KeySpecial::F10)),
    /*  068  */
    KeyMappingEntry::new(Key::Special(KeySpecial::F11), Key::Special(KeySpecial::F11)),
    /*  069  */
    KeyMappingEntry::new(Key::Special(KeySpecial::F12), Key::Special(KeySpecial::F12)),
    // /*  070  */ KeyMappingEntry::new(Key::Ascii(b'*'), Key::Ascii(b'*')),
    /*  071  */
    KeyMappingEntry::new(
        Key::Special(KeySpecial::ScrollLock),
        Key::Special(KeySpecial::ScrollLock),
    ),
    /*  072  */
    KeyMappingEntry::new(
        Key::Special(KeySpecial::None),
        Key::Special(KeySpecial::None),
    ),
    /*  073  */
    KeyMappingEntry::new(
        Key::Special(KeySpecial::None),
        Key::Special(KeySpecial::None),
    ),
    /*  074  */
    KeyMappingEntry::new(
        Key::Special(KeySpecial::Insert),
        Key::Special(KeySpecial::Insert),
    ),
    /*  075  */
    KeyMappingEntry::new(
        Key::Special(KeySpecial::Home),
        Key::Special(KeySpecial::Home),
    ),
    /*  076  */
    KeyMappingEntry::new(
        Key::Special(KeySpecial::PageUp),
        Key::Special(KeySpecial::PageUp),
    ),
    /*  077  */
    KeyMappingEntry::new(
        Key::Special(KeySpecial::Delete),
        Key::Special(KeySpecial::Delete),
    ),
    /*  078  */
    KeyMappingEntry::new(Key::Special(KeySpecial::End), Key::Special(KeySpecial::End)),
    /*  079  */
    KeyMappingEntry::new(
        Key::Special(KeySpecial::PageDown),
        Key::Special(KeySpecial::PageDown),
    ),
    /*  080  */
    KeyMappingEntry::new(
        Key::Special(KeySpecial::Right),
        Key::Special(KeySpecial::Right),
    ),
    /*  081  */
    KeyMappingEntry::new(
        Key::Special(KeySpecial::Left),
        Key::Special(KeySpecial::Left),
    ),
    /*  082  */
    KeyMappingEntry::new(
        Key::Special(KeySpecial::Down),
        Key::Special(KeySpecial::Down),
    ),
    /*  083  */
    KeyMappingEntry::new(Key::Special(KeySpecial::Up), Key::Special(KeySpecial::Up)),
    /*  084  */
    KeyMappingEntry::new(
        Key::Special(KeySpecial::NumLock),
        Key::Special(KeySpecial::NumLock),
    ),
    /*  085  */ KeyMappingEntry::new(Key::Ascii(b'/'), Key::Ascii(b'/')),
    /*  086  */ KeyMappingEntry::new(Key::Ascii(b'*'), Key::Ascii(b'*')),
    /*  087  */ KeyMappingEntry::new(Key::Ascii(b'-'), Key::Ascii(b'-')),
    /*  088  */ KeyMappingEntry::new(Key::Ascii(b'+'), Key::Ascii(b'+')),
    /*  089  */
    KeyMappingEntry::new(
        Key::Special(KeySpecial::Enter),
        Key::Special(KeySpecial::Enter),
    ),
    /*  090  */ KeyMappingEntry::new(Key::Ascii(b'1'), Key::Special(KeySpecial::End)),
    /*  091  */ KeyMappingEntry::new(Key::Ascii(b'2'), Key::Special(KeySpecial::Down)),
    /*  092  */ KeyMappingEntry::new(Key::Ascii(b'3'), Key::Special(KeySpecial::PageDown)),
    /*  093  */ KeyMappingEntry::new(Key::Ascii(b'4'), Key::Special(KeySpecial::Left)),
    /*  094  */ KeyMappingEntry::new(Key::Ascii(b'5'), Key::Special(KeySpecial::Center)),
    /*  095  */ KeyMappingEntry::new(Key::Ascii(b'6'), Key::Special(KeySpecial::Right)),
    /*  096  */ KeyMappingEntry::new(Key::Ascii(b'7'), Key::Special(KeySpecial::Home)),
    /*  097  */ KeyMappingEntry::new(Key::Ascii(b'8'), Key::Special(KeySpecial::Up)),
    /*  098  */ KeyMappingEntry::new(Key::Ascii(b'9'), Key::Special(KeySpecial::PageUp)),
    /*  099  */ KeyMappingEntry::new(Key::Ascii(b'0'), Key::Special(KeySpecial::Insert)),
    /*  100  */ KeyMappingEntry::new(Key::Ascii(b'.'), Key::Special(KeySpecial::Delete)),
    /*  101  */
    KeyMappingEntry::new(
        Key::Special(KeySpecial::None),
        Key::Special(KeySpecial::None),
    ),
    /*  102  */
    KeyMappingEntry::new(
        Key::Special(KeySpecial::None),
        Key::Special(KeySpecial::None),
    ),
    /*  103  */ KeyMappingEntry::new(Key::Ascii(b'='), Key::Ascii(b'=')),
];

#[derive(Clone, Copy)]
pub struct USBKeycode(u8);

impl Keycode for USBKeycode {
    fn is_alpha(&self) -> bool {
        self.0 >= 0x04 && self.0 <= 0x1D
    }

    fn is_common_num(&self) -> bool {
        self.0 >= 0x1E && self.0 <= 0x27
    }

    fn is_num_pad(&self) -> bool {
        self.0 >= 0x59 && self.0 <= 0x63
    }
}

trait KeyboardSubscriber {
    fn subscribe(&self, modified_bit: u8, keycode: Key);
}

impl<F> KeyboardSubscriber for F
where
    F: Fn(u8, Key),
{
    fn subscribe(&self, modified_bit: u8, keycode: Key) {
        self(modified_bit, keycode)
    }
}

#[derive(Clone)]
pub struct USBKeyboardDriver {
    prev_buff: [u8; 8],
    data_buff: [u8; 8],
    subsrcribe: Rc<dyn KeyboardSubscriber>,
    manager: KeyboardManager,
}

impl USBKeyboardDriver {
    pub fn new<F>(subscribe: F) -> Self
    where
        F: Fn(u8, Key) + 'static,
    {
        Self {
            prev_buff: [0u8; 8],
            data_buff: [0u8; 8],
            subsrcribe: Rc::new(subscribe),
            manager: KeyboardManager::new(),
        }
    }

    pub fn keycodes(&mut self) -> Vec<Key> {
        let pressed: Vec<Key> = self.data_buff[2..]
            .iter()
            .filter(|&keycode| !self.prev_buff[2..].contains(keycode))
            .map(|&keycode| self.keycode(keycode))
            .collect();
        // for key in pressed.iter() {
        //     self.manager.update_key_status(key);
        // }
        self.prev_buff = self.data_buff;
        pressed
    }

    fn keycode(&self, keycode: u8) -> Key {
        match KEY_MAPPING_TABLE.get(keycode as usize) {
            Some(entry) => {
                let shift_pressed =
                    self.data_buff[0] & (COMBINE_KEY_LSHIFT | COMBINE_KEY_RSHIFT) != 0;
                let keyboard_status = self
                    .manager
                    .is_combined_code(USBKeycode(keycode), shift_pressed);
                if keyboard_status {
                    entry.combined()
                } else {
                    entry.normal()
                }
            }
            None => Key::Special(KeySpecial::None),
        }
    }
}

impl ClassDriverOperate for USBKeyboardDriver {
    fn on_data_received(&mut self) -> Result<(), ()> {
        debug!("Keyboard {:02X?}", self.data_buff);
        for key in self.keycodes().iter() {
            self.manager.update_key_status(key);
            self.subsrcribe.subscribe(self.data_buff[0], *key);
        }
        Ok(())
    }

    fn data_buffer_addr(&self) -> u64 {
        self.data_buff.as_ptr() as u64
    }

    fn data_buffer_len(&self) -> u32 {
        8
    }

    fn driver_type(&self) -> DriverType {
        DriverType::Keyboard
    }
}
