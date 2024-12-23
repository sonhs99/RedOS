use core::fmt::Write;
use core::{fmt, str};

use alloc::string::ToString;

use crate::{
    device::driver::keyboard::keycode::{Key, KeySpecial},
    graphic::PixelColor,
    window::{
        draw::{draw_rect, draw_str, Point},
        event::{Event, EventType, KeyEvent},
        frame::WindowFrame,
        Movable, Writable,
    },
};

const CURSOR: [u8; 16] = [
    0b0000_0000,
    0b0000_0000,
    0b0000_0000,
    0b0000_0000,
    0b0000_0000,
    0b0000_0000,
    0b0000_0000,
    0b0000_0000,
    0b0000_0000,
    0b0000_0000,
    0b0000_0000,
    0b0000_0000,
    0b0000_0000,
    0b1111_1111,
    0b1111_1111,
    0b1111_1111,
];

const PROMPT: &str = "> ";
const COMMAND_BUFFER_LENGTH: usize = 300;

struct Shell {
    width: usize,
    height: usize,
    curser: usize,
    command_buffer: [u8; COMMAND_BUFFER_LENGTH],
}

impl Shell {
    pub fn new(resolution: (usize, usize)) -> Self {
        Self {
            width: resolution.0,
            height: resolution.1,
            curser: 0,
            command_buffer: [0; COMMAND_BUFFER_LENGTH],
        }
    }

    pub fn clear_command(&mut self) {
        self.command_buffer.fill(0);
        self.curser = 0;
    }

    pub fn write_char(&mut self, c: u8) -> Option<usize> {
        if self.curser < COMMAND_BUFFER_LENGTH {
            self.command_buffer[self.curser] = c;
            self.curser += 1;
            Some(self.curser)
        } else {
            None
        }
    }

    pub fn remove_char(&mut self) -> Option<usize> {
        if self.curser != 0 {
            self.command_buffer[self.curser] = 0;
            self.curser -= 1;
            Some(self.curser)
        } else {
            None
        }
    }

    pub fn get_command_str(&self) -> &str {
        str::from_utf8(&self.command_buffer[..self.curser]).expect("Cannot convert str")
    }
}

struct ShellWriter<Writer: Writable + Movable> {
    writer: Writer,
    cursor: usize,
    width: usize,
    height: usize,
}

impl<Writer: Writable + Movable> ShellWriter<Writer> {
    pub fn new(resolution: (usize, usize), writer: Writer) -> Self {
        Self {
            writer,
            cursor: 0,
            width: resolution.0 / 8,
            height: resolution.1 / 16,
        }
    }

    pub fn put_string(&mut self, s: &[u8]) {
        for &c in s {
            match c {
                0x20..=0x7e => self.put_char(c),
                b'\n' => {
                    self.put_char(b' ');
                    if self.cursor >= (self.width - 1) * self.height {
                        self.newline();
                    } else {
                        self.cursor += (self.width - self.cursor % self.width);
                    }
                }
                _ => self.put_char(0xfe),
            }
        }
    }

    pub fn put_char(&mut self, c: u8) {
        if self.cursor >= self.width * self.height {
            self.newline();
        }
        let column = self.cursor % self.width;
        let row = self.cursor / self.width;
        draw_str(
            Point(8 * column, 16 * row),
            &(c as char).to_string(),
            PixelColor::White,
            PixelColor::Black,
            &mut self.writer,
        );
        self.cursor += 1
    }

    pub fn cls(&mut self) {
        draw_rect(
            Point(0, 0),
            Point(self.width, self.height),
            PixelColor::Black,
            true,
            &mut self.writer,
        );
        self.cursor = 0;
    }

    fn newline(&mut self) {
        self.cursor -= self.width;
        self.writer.move_(0, -16);
        draw_rect(
            Point(0, 16 * (self.height - 1) as usize),
            Point(8 * self.width, 16 * self.height),
            PixelColor::Black,
            true,
            &mut self.writer,
        );
    }

    pub fn remove_char(&mut self) {
        self.cursor -= 1;
        let column = self.cursor % self.width;
        let row = self.cursor / self.width;
        draw_str(
            Point(8 * column, 16 * row),
            "  ",
            PixelColor::White,
            PixelColor::Black,
            &mut self.writer,
        );
    }

    pub fn put_curser(&mut self) {
        let column = self.cursor % self.width;
        let row = self.cursor / self.width;
        print_curser(Point(8 * column, 16 * row), &mut self.writer);
    }
}

impl<Writer: Writable + Movable> fmt::Write for ShellWriter<Writer> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.put_string(s.as_bytes());
        Ok(())
    }
}

pub fn print_curser(offset: Point, writer: &mut dyn Writable) {
    for y in 0..16 {
        for x in 0..8 {
            let color = if CURSOR[y] >> (7 - x) & 0x01 != 0 {
                PixelColor::White
            } else {
                PixelColor::Black
            };
            writer.write(offset.0 + x, offset.1 + y, color);
        }
    }
}

pub fn start_shell() {
    let mut shell = Shell::new((640, 400));
    let mut window = WindowFrame::new(640, 400, "Shell");
    let mut shell_writer = ShellWriter::new((640, 400), window.body());
    let mut current_curser = 0;
    let mut pos = Point(2, 0);
    window.set_background(PixelColor::Black);
    write!(&mut shell_writer, "{}", PROMPT);
    loop {
        if let Some(event) = window.pop_event() {
            if let EventType::Keyboard(key_event) = event.event() {
                if let KeyEvent::Pressed(key) = key_event {
                    match key {
                        Key::Ascii(c) => {
                            if let Some(next_curser) = shell.write_char(c) {
                                write!(&mut shell_writer, "{}", c as char);
                                shell_writer.put_curser();
                            }
                        }
                        Key::Special(key_special) => match key_special {
                            KeySpecial::Enter => {
                                let command = shell.get_command_str();
                                write!(&mut shell_writer, "\n{PROMPT}");
                                shell_writer.put_curser();
                                shell.clear_command();
                            }
                            KeySpecial::Backspace => {
                                if let Some(next_curser) = shell.remove_char() {
                                    shell_writer.remove_char();
                                    shell_writer.put_curser();
                                }
                            }
                            KeySpecial::Left | KeySpecial::Right => {}
                            _ => {}
                        },
                    }
                }
            }
        }
    }
}
