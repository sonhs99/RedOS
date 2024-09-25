use core::{fmt, ptr::write_bytes};

use crate::{
    font::write_ascii,
    graphic::{get_graphic, GraphicWriter, PixelColor},
    interrupt::{apic::LocalAPICRegisters, without_interrupts},
    sync::{Mark, Mutex, OnceLock},
    window::{
        draw::{draw_rect, Point},
        frame::WindowFrame,
        request_update_by_id, Movable, WindowWriter,
    },
};

use log::{Level, Log};

pub static CONSOLE: OnceLock<Mark<Mutex<Console>>> = OnceLock::new();
static WINDOW_WRITER: OnceLock<Mutex<WindowFrame>> = OnceLock::new();
pub static LOGGER: ConsoleLogger = ConsoleLogger;

pub struct Console {
    bg_color: PixelColor,
    fg_color: PixelColor,
    buffer: [[u8; Console::Columns]; Console::Rows],
    cursor_column: u64,
    cursor_row: u64,
}

pub struct ConsoleLogger;

impl Console {
    pub const Rows: usize = 25;
    pub const Columns: usize = 80;
    pub const fn new(bg_color: PixelColor, fg_color: PixelColor) -> Self {
        Self {
            bg_color,
            fg_color,
            buffer: [[0u8; Console::Columns]; Console::Rows],
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
                if self.cursor_column > Console::Columns as u64 - 1 {
                    self.newline();
                }
                match WINDOW_WRITER.get() {
                    Some(writer) => write_ascii(
                        8 * self.cursor_column,
                        16 * self.cursor_row,
                        c,
                        self.fg_color,
                        Some(self.bg_color),
                        &mut writer.lock().body(),
                    ),
                    None => write_ascii(
                        8 * self.cursor_column,
                        16 * self.cursor_row,
                        c,
                        self.fg_color,
                        Some(self.bg_color),
                        &mut get_graphic().lock(),
                    ),
                }
                self.buffer[self.cursor_row as usize][self.cursor_column as usize] = c;
                self.cursor_column += 1
            }
        }
    }

    pub fn cls(&mut self) {
        for y in 0..self.cursor_row {
            self.buffer[y as usize] = [0u8; Console::Columns];
        }
        for x in 0..self.cursor_column {
            self.buffer[self.cursor_row as usize][x as usize] = 0;
        }
        self.cursor_column = 0;
        self.cursor_row = 0;
    }

    fn newline(&mut self) {
        self.cursor_column = 0;
        if (self.cursor_row as usize) < Console::Rows - 1 {
            self.cursor_row += 1
        } else {
            match WINDOW_WRITER.get() {
                Some(writer) => {
                    let mut body = writer.lock().body();
                    body.move_(0, -16);
                    draw_rect(
                        Point(0, 16 * self.cursor_row as usize),
                        Point(8 * Self::Columns, 16 * (self.cursor_row as usize + 1)),
                        self.bg_color,
                        true,
                        &mut body,
                    );
                }
                None => {
                    for row in 0..self.cursor_row as usize {
                        for column in 0..Console::Columns {
                            if self.buffer[row][column] == 0 && self.buffer[row + 1][column] == 0 {
                                break;
                            }
                            let c = self.buffer[row + 1][column];
                            self.buffer[row][column] = c;
                            write_ascii(
                                8 * column as u64,
                                16 * row as u64,
                                c,
                                self.fg_color,
                                Some(self.bg_color),
                                &mut get_graphic().lock(),
                            )
                        }
                    }
                    for column in 0..Console::Columns {
                        self.buffer[Console::Rows - 1][column] = 0;
                        write_ascii(
                            8 * column as u64,
                            16 * self.cursor_row,
                            b' ',
                            self.fg_color,
                            Some(self.bg_color),
                            &mut get_graphic().lock(),
                        )
                    }
                }
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
    CONSOLE.get_or_init(|| Mark::new(Mutex::new(Console::new(bg_color, fg_color))));
    let _ = log::set_logger(&LOGGER).map(|()| log::set_max_level(log::LevelFilter::Debug));
}

pub fn alloc_window(writer: WindowFrame) {
    WINDOW_WRITER.get_or_init(|| Mutex::new(writer));
    CONSOLE.skip().lock().cls();
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
            let apic_id = LocalAPICRegisters::default().local_apic_id().id();
            println!("[{:>5}: {}]: {}", record.level(), apic_id, record.args());
        }
    }

    fn flush(&self) {
        CONSOLE.skip().lock().cls();
    }
}

#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    use core::fmt::Write;
    {
        CONSOLE.skip().lock().write_fmt(args).unwrap();
    }
    if let Some(writer) = WINDOW_WRITER.get() {
        let id = writer.lock().window_id();
        request_update_by_id(id);
    }
}
