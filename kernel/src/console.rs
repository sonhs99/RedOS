use core::{fmt, ptr::write_bytes};

use crate::{
    font::write_ascii,
    graphic::{get_graphic, GraphicWriter, PixelColor},
    interrupt::without_interrupts,
    sync::{Mutex, OnceLock},
};

use log::{Level, Log};

pub static CONSOLE: OnceLock<Mutex<Console>> = OnceLock::new();
pub static LOGGER: ConsoleLogger = ConsoleLogger;

pub struct Console {
    bg_color: PixelColor,
    fg_color: PixelColor,
    buffer: [[u8; Console::Columns + 1]; Console::Rows],
    cursor_column: u64,
    cursor_row: u64,
}

pub struct ConsoleLogger;

impl Console {
    pub const Rows: usize = 50;
    pub const Columns: usize = 80;
    pub const fn new(bg_color: PixelColor, fg_color: PixelColor) -> Self {
        Self {
            bg_color,
            fg_color,
            buffer: [[0u8; Console::Columns + 1]; Console::Rows],
            cursor_column: 0,
            cursor_row: 0,
        }
    }

    pub fn put_string(&mut self, s: &[u8]) {
        for &c in s {
            match c {
                0x20..=0x7e | b'\n' => self.put_char(c),
                _ => self.put_char(0xfe),
            }
        }
    }

    pub fn put_char(&mut self, c: u8) {
        match c {
            b'\n' => self.newline(),
            _ => {
                if self.cursor_column >= Console::Columns as u64 {
                    self.newline();
                }
                write_ascii(
                    8 * self.cursor_column,
                    16 * self.cursor_row,
                    c,
                    self.fg_color,
                    self.bg_color,
                );
                self.buffer[self.cursor_row as usize][self.cursor_column as usize] = c;
                self.cursor_column += 1
            }
        }
    }

    fn cls(&mut self) {}

    fn newline(&mut self) {
        self.cursor_column = 0;
        if (self.cursor_row as usize) < Console::Rows - 1 {
            self.cursor_row += 1
        } else {
            for y in 0..16 * Console::Rows {
                for x in 0..8 * Console::Columns {
                    get_graphic().lock().write(x, y, self.bg_color);
                }
            }
            for row in 0..(Console::Rows - 1) {
                for column in 0..(Console::Columns + 1) {
                    let c = self.buffer[row + 1][column];
                    self.buffer[row][column] = c;
                    write_ascii(
                        8 * column as u64,
                        16 * row as u64,
                        c,
                        self.fg_color,
                        self.bg_color,
                    );
                }
            }
            for column in 0..(Console::Columns + 1) {
                self.buffer[Console::Rows - 1][column] = 0;
            }
        }
    }
}

impl fmt::Write for Console {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        self.put_string(s.as_bytes());
        Ok(())
    }
}

pub fn init_console(bg_color: PixelColor, fg_color: PixelColor) {
    CONSOLE.get_or_init(|| Mutex::new(Console::new(bg_color, fg_color)));
    let _ = log::set_logger(&LOGGER).map(|()| log::set_max_level(log::LevelFilter::Debug));
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::console::_print(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}

impl Log for ConsoleLogger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        metadata.level() <= Level::Trace
    }

    fn log(&self, record: &log::Record) {
        if self.enabled(record.metadata()) {
            println!("[{:>5}]: {}", record.level(), record.args());
        }
    }

    fn flush(&self) {}
}

#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    use core::fmt::Write;
    without_interrupts(|| {
        CONSOLE.lock().write_fmt(args).unwrap();
    });
}
