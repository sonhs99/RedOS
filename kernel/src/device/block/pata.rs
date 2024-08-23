use core::{
    hint::{self, black_box},
    ops::Not,
    ptr::{slice_from_raw_parts, slice_from_raw_parts_mut},
    str,
    sync::atomic::AtomicBool,
};

use log::{debug, error};

use crate::{
    device::Port,
    sync::{Mutex, OnceLock},
    timer::{convert_ms_to_tick, read_pm_count, sleep, wait_ms, wait_us},
};

use super::{Block, BlockIO};

const PATA_PORT_PRIMARY_BASE: u16 = 0x01F0;
const PATA_PORT_SECONDARY_BASE: u16 = 0x0170;

const PATA_DRIVEANDHEAD_LBA: u8 = 0xE0;
const PATA_DRIVEANDHEAD_SLAVE: u8 = 0x10;

const PATA_WAIT_TIME: u32 = 500;

const PATA_COMMAND_INFO: u8 = 0xEC;
const PATA_COMMAND_READ: u8 = 0x20;
const PATA_COMMAND_WRITE: u8 = 0x30;

pub struct HDD {
    primary: bool,
    master: bool,
    can_write: bool,
    info: HDDInformation,
}

pub struct Pata {
    primary_interrupt: bool,
    secondary_interrupt: bool,
}

pub struct PataBus {
    data: Port,
    error: Port,
    sector_count: Port,
    sector_num: Port,
    cylinder_lsb: Port,
    cylinder_msb: Port,
    drive_head: Port,
    status: Port,
    command: Port,
    digital_int: Port,
    address: Port,
}

struct PataStatus(u8);

impl PataStatus {
    pub const fn busy(&self) -> bool {
        self.0 & 0x80 != 0
    }

    pub const fn ready(&self) -> bool {
        self.0 & 0x40 != 0
    }

    pub const fn fault(&self) -> bool {
        self.0 & 0x20 != 0
    }

    pub const fn seek_complete(&self) -> bool {
        self.0 & 0x10 != 0
    }

    pub const fn request(&self) -> bool {
        self.0 & 0x08 != 0
    }

    pub const fn correctable_data_error(&self) -> bool {
        self.0 & 0x04 != 0
    }

    pub const fn index(&self) -> bool {
        self.0 & 0x02 != 0
    }

    pub const fn error(&self) -> bool {
        self.0 & 0x01 != 0
    }
}

impl PataBus {
    pub const fn new(base: u16) -> Self {
        Self {
            data: Port::new(base + 0x000),
            error: Port::new(base + 0x001),
            sector_count: Port::new(base + 0x002),
            sector_num: Port::new(base + 0x003),
            cylinder_lsb: Port::new(base + 0x004),
            cylinder_msb: Port::new(base + 0x005),
            drive_head: Port::new(base + 0x006),
            status: Port::new(base + 0x007),
            command: Port::new(base + 0x007),
            digital_int: Port::new(base + 0x206),
            address: Port::new(base + 0x207),
        }
    }

    pub fn status(&self) -> PataStatus {
        PataStatus(self.status.in8())
    }
}

#[derive(Clone, Copy)]
#[repr(C, packed(1))]
pub struct HDDInformation {
    configuration: u16,

    num_of_cylinder: u16,
    _reserved1: u16,

    num_of_head: u16,
    unformatted_byte_per_track: u16,
    unformatted_byte_per_sector: u16,

    num_of_sector_per_cylinder: u16,
    inter_sector_gap: u16,
    byte_in_phase_lock: u16,
    num_of_vender_unique_status_word: u16,

    serial_number: [u16; 10],
    controller_type: u16,
    buffer_size: u16,
    num_of_esc_bytes: u16,
    firmware_revision: [u16; 4],

    model_number: [u16; 20],
    _reserved2: [u16; 13],

    total_sectors: u32,
    _reserved3: [u16; 194],
}

impl HDDInformation {
    pub const fn empty() -> Self {
        Self {
            configuration: 0,
            num_of_cylinder: 0,
            _reserved1: 0,
            num_of_head: 0,
            unformatted_byte_per_track: 0,
            unformatted_byte_per_sector: 0,
            num_of_sector_per_cylinder: 0,
            inter_sector_gap: 0,
            byte_in_phase_lock: 0,
            num_of_vender_unique_status_word: 0,
            serial_number: [0; 10],
            controller_type: 0,
            buffer_size: 0,
            num_of_esc_bytes: 0,
            firmware_revision: [0; 4],
            model_number: [0; 20],
            _reserved2: [0; 13],
            total_sectors: 0,
            _reserved3: [0; 194],
        }
    }

    pub fn as_ptr(&self) -> *const u16 {
        (self as *const Self).cast::<u16>()
    }

    pub fn as_mut_ptr(&mut self) -> *mut u16 {
        (self as *mut Self).cast::<u16>()
    }

    pub fn serial_number(&mut self) -> &mut [u16] {
        unsafe {
            let ptr = self.as_mut_ptr().add(10);
            &mut *slice_from_raw_parts_mut(ptr, 10)
        }
    }

    pub fn serial_number_str(&self) -> &str {
        unsafe {
            let ptr = self.as_ptr().add(10).cast::<u8>();
            let slice = &*slice_from_raw_parts(ptr, 20);
            str::from_utf8(slice).unwrap()
        }
    }

    pub fn model_number(&mut self) -> &mut [u16] {
        unsafe {
            let ptr = self.as_mut_ptr().add(27);
            &mut *slice_from_raw_parts_mut(ptr, 20)
        }
    }

    pub fn model_number_str(&self) -> &str {
        unsafe {
            let ptr = self.as_ptr().add(27).cast::<u8>();
            let slice = &*slice_from_raw_parts(ptr, 40);
            str::from_utf8(slice).unwrap()
        }
    }
}

impl Pata {
    pub fn new() -> Self {
        PATA_BUS[0].digital_int.out8(0);
        PATA_BUS[1].digital_int.out8(0);
        Self {
            primary_interrupt: false,
            secondary_interrupt: false,
        }
    }

    pub fn set_interrupt_flag(&mut self, primary: bool, flag: bool) {
        if primary {
            self.primary_interrupt = flag;
        } else {
            self.secondary_interrupt = flag;
        }
    }

    pub fn device(&mut self, index: u8) -> Result<HDD, ()> {
        let primary = index / 2 == 0;
        let master = index % 2 == 0;
        let mut hdd = HDD {
            primary,
            master,
            can_write: true,
            info: HDDInformation::empty(),
        };

        hdd.wait_hdd_no_busy(primary)?;

        let driver_flag = PATA_DRIVEANDHEAD_LBA | if master { 0 } else { PATA_DRIVEANDHEAD_SLAVE };

        PATA_BUS[!primary as usize].drive_head.out8(driver_flag);

        hdd.wait_hdd_ready(primary)?;

        self.set_interrupt_flag(primary, false);
        PATA_BUS[!primary as usize].command.out8(PATA_COMMAND_INFO);
        if hdd.wait_hdd_interrupt(primary).is_err() || PATA_BUS[!primary as usize].status().error()
        {
            return Err(());
        }
        unsafe {
            let info_ptr = hdd.info.as_mut_ptr();
            for idx in 0..(512 / 2) {
                *info_ptr.add(idx) = PATA_BUS[!primary as usize].data.in16();
            }

            for word in hdd.info.model_number().iter_mut() {
                *word = *word << 8 | *word >> 8;
            }

            for word in hdd.info.serial_number().iter_mut() {
                *word = *word << 8 | *word >> 8;
            }
        }
        Ok(hdd)
    }
}

impl HDD {
    fn wait_hdd_no_busy(&self, primary: bool) -> Result<(), ()> {
        let start = read_pm_count();
        while read_pm_count().wrapping_sub(start) <= convert_ms_to_tick(PATA_WAIT_TIME) {
            let status = PATA_BUS[!primary as usize].status();
            if !status.busy() {
                return Ok(());
            }
            sleep(1);
        }
        Err(())
    }

    fn wait_hdd_ready(&self, primary: bool) -> Result<(), ()> {
        let start = read_pm_count();
        while read_pm_count().wrapping_sub(start) <= convert_ms_to_tick(PATA_WAIT_TIME) {
            let status = PATA_BUS[!primary as usize].status();
            if status.ready() {
                return Ok(());
            }
            sleep(1);
        }
        Err(())
    }

    fn wait_hdd_interrupt(&self, primary: bool) -> Result<(), ()> {
        let start = read_pm_count();
        while read_pm_count().wrapping_sub(start) <= convert_ms_to_tick(PATA_WAIT_TIME) {
            {
                let pata = PATA.lock();
                if hint::black_box(primary == pata.primary_interrupt)
                    || hint::black_box(!primary == pata.secondary_interrupt)
                {
                    return Ok(());
                }
            }

            sleep(1);
        }
        Err(())
    }

    pub fn info(&self) -> HDDInformation {
        self.info
    }

    pub fn read_block(&self, address: u32, buffer: &mut [Block<512>]) -> Result<usize, usize> {
        if self.info.total_sectors <= address + buffer.len() as u32 {
            return Err(0);
        }

        let drive = if self.master { 0xE0 } else { 0xF0 };
        let drive_head_value = drive | ((address >> 24) & 0x0F) as u8;

        self.wait_hdd_no_busy(self.primary).map_err(|err| 0usize)?;
        PATA_BUS[!self.primary as usize]
            .sector_count
            .out8(buffer.len() as u8);
        PATA_BUS[!self.primary as usize]
            .sector_num
            .out8(address as u8);
        PATA_BUS[!self.primary as usize]
            .cylinder_lsb
            .out8((address >> 8) as u8);
        PATA_BUS[!self.primary as usize]
            .cylinder_msb
            .out8((address >> 16) as u8);
        PATA_BUS[!self.primary as usize]
            .drive_head
            .out8(drive_head_value);

        self.wait_hdd_ready(self.primary).map_err(|err| 0usize)?;
        PATA.lock().set_interrupt_flag(self.primary, false);
        PATA_BUS[!self.primary as usize]
            .command
            .out8(PATA_COMMAND_READ);
        let mut count = 0usize;
        for block in buffer {
            let status = PATA_BUS[!self.primary as usize].status();
            let status = PATA_BUS[!self.primary as usize].status();
            if status.error() {
                error!("PATA: Error Occur");
                return Err(count);
            }
            // debug!("req={}", status.request());

            if !status.request() {
                let wait_result = self
                    .wait_hdd_interrupt(self.primary)
                    .inspect_err(|err| {
                        PATA.lock().set_interrupt_flag(self.primary, false);
                        error!("PATA: Interrupt Not Occur");
                    })
                    .map_err(|err| count)?;
                PATA.lock().set_interrupt_flag(self.primary, false);
            }
            wait_us(400);
            for index in 0..512 / 2 {
                *block.get_mut(index) = PATA_BUS[!self.primary as usize].data.in16();
            }
            count += 1;
        }
        Ok(count)
    }

    pub fn write_block(&self, address: u32, buffer: &[Block<512>]) -> Result<usize, usize> {
        if !self.can_write || self.info.total_sectors <= address + buffer.len() as u32 {
            return Err(0);
        }

        let drive = if self.master { 0xE0 } else { 0xF0 };
        let drive_head_value = drive | ((address >> 24) & 0x0F) as u8;

        self.wait_hdd_no_busy(self.primary).map_err(|err| 0usize)?;
        PATA_BUS[!self.primary as usize]
            .sector_count
            .out8(buffer.len() as u8);
        PATA_BUS[!self.primary as usize]
            .sector_num
            .out8(address as u8);
        PATA_BUS[!self.primary as usize]
            .cylinder_lsb
            .out8((address >> 8) as u8);
        PATA_BUS[!self.primary as usize]
            .cylinder_msb
            .out8((address >> 16) as u8);
        PATA_BUS[!self.primary as usize]
            .drive_head
            .out8(drive_head_value);

        self.wait_hdd_ready(self.primary).map_err(|err| 0usize)?;
        PATA_BUS[!self.primary as usize]
            .command
            .out8(PATA_COMMAND_WRITE);

        loop {
            let status = PATA_BUS[!self.primary as usize].status();
            if status.error() {
                return Err(0);
            }
            if status.request() {
                break;
            }
            sleep(1);
        }

        let mut count = 0;
        for block in buffer {
            PATA.lock().set_interrupt_flag(self.primary, false);
            for index in 0..512 / 2 {
                PATA_BUS[!self.primary as usize]
                    .data
                    .out16(*block.get(index));
            }
            count += 1;
            let status = PATA_BUS[!self.primary as usize].status();
            if status.error() {
                return Err(count);
            }
            // debug!("req={}", status.request());
            if !status.request() {
                let wait_result = self
                    .wait_hdd_interrupt(self.primary)
                    .inspect_err(|err| {
                        PATA.lock().set_interrupt_flag(self.primary, false);
                        error!("PATA: Interrupt Not Occur");
                    })
                    .map_err(|err| count)?;
                PATA.lock().set_interrupt_flag(self.primary, false);
            }
            wait_us(400);
        }
        Ok(count)
    }
}

impl BlockIO for HDD {
    fn read(&mut self, address: u32, buffer: &mut [Block<512>]) -> Result<usize, usize> {
        self.read_block(address, buffer)
    }

    fn write(&mut self, address: u32, buffer: &[Block<512>]) -> Result<usize, usize> {
        self.write_block(address, buffer)
    }

    fn max_addr(&self) -> u32 {
        self.info.total_sectors
    }
}

static PATA_BUS: [PataBus; 2] = [
    PataBus::new(PATA_PORT_PRIMARY_BASE),
    PataBus::new(PATA_PORT_SECONDARY_BASE),
];

static PATA: OnceLock<Mutex<Pata>> = OnceLock::new();

pub fn init_pata() {
    PATA.get_or_init(|| Mutex::new(Pata::new()));
}

pub fn set_interrupt_flag(primary: bool) {
    PATA.lock().set_interrupt_flag(primary, true);
}

pub fn get_device(index: u8) -> Result<HDD, ()> {
    PATA.lock().device(index)
}
