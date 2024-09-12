use core::slice;
use core::str;

use alloc::borrow::ToOwned;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec;
use alloc::{boxed::Box, vec::Vec};
use ascii::{AsciiStr, AsciiString};
use log::debug;

use crate::cache::Cache;
use crate::device::block::{Block, BlockIO};
use crate::utility::ceil;

use super::{
    CommonFATHeader, DirectoryDescriptor, DirectoryEntry, FileDescriptor, FileSystem,
    FAT_DIR_ATTRIBUTE_DIR, FAT_DIR_ATTRIBUTE_FILE, FAT_END_OF_CLUSTER,
    FAT_MAX_DIRECTORY_ENTRY_COUNT, FAT_SECTOR_PER_CLUSTER,
};

const FAT16_RESERVED_SECTOR_COUNT: u32 = 1;
const FAT16_CACHE_SIZE: usize = 128;
const FAT16_CLUSTER_ENTRY_PER_SECTOR: u32 = 512 / 2;
const FAT16_END_OF_CLUSTER: u32 = 0xFFF8;

#[repr(C, packed)]
struct FAT16Header {
    common: CommonFATHeader,
    drive_number: u8,
    _reserved1: u8,
    boot_signature: u8,
    serial_number: u32,
    volume_label: [u8; 11],
    _reserved2: [u8; 458],
}

impl FAT16Header {
    pub const fn empty() -> Self {
        Self {
            common: CommonFATHeader::empty(),
            drive_number: 0x80,
            _reserved1: 0u8,
            boot_signature: 0u8,
            serial_number: 0u32,
            volume_label: [0u8; 11],
            _reserved2: [0; 458],
        }
    }

    pub fn as_block(&self) -> &Block<512> {
        unsafe { &*(self as *const Self).cast::<Block<512>>() }
    }
}
pub struct FAT16 {
    start_addr: u32,
    volume_size: u32,
    reserved_sector_count: u32,

    byte_per_sector: u16,
    sector_per_cluster: u8,

    fat_addr: u32,
    fat2_addr: u32,
    fat_size: u32,

    data_addr: u32,

    current_cluster: u32,
    root_directory_cluster: u32,

    use_cache: bool,
    fat_cache: Cache<Block<512>>,
    cluster_cache: Cache<Block<512>>,
}

impl FAT16 {
    pub fn mount(
        device: &mut dyn BlockIO,
        start_addr: u32,
        volume_size: u32,
        use_cache: bool,
    ) -> Result<Self, ()> {
        let mut buffer = vec![Block::empty()];
        device.read(start_addr, &mut buffer).map_err(|err| ())?;
        let header = buffer[0].convert::<FAT16Header>();
        let byte_per_sector = header.common.byte_per_sector;
        let sector_per_cluster = header.common.sector_per_clustor;
        let reserved_sector_count = header.common.reserved_sector_count;

        let total_sector = if header.common.total_sector16 != 0 {
            header.common.total_sector16 as u32
        } else {
            header.common.total_sector32
        };
        let fat_size = header.common.fat_size16 as u32;
        let root_directory_cluster = 0;

        // debug!("Jmp Boot Code  : {:0X?}", header.common.jmp_boot_code);
        // debug!(
        //     "Name String    : {}",
        //     AsciiStr::from_ascii(&header.common.oem_id).unwrap()
        // );
        // debug!("Byte/Sector    : {byte_per_sector}");
        // debug!("Sector/Cluster : {sector_per_cluster}");
        // debug!("Reserved Sector: {reserved_sector_count:#X}");
        // debug!("Total Sector   : {total_sector:#X}");
        // debug!("FAT Size       : {fat_size:#X}");
        // debug!("Root Cluster   : {root_directory_cluster:#X}");

        Ok(Self {
            start_addr,
            volume_size: total_sector,
            reserved_sector_count: reserved_sector_count as u32,
            byte_per_sector,
            sector_per_cluster,
            fat_addr: start_addr + 32,
            fat_size,
            fat2_addr: start_addr + reserved_sector_count as u32 + fat_size,
            data_addr: start_addr + reserved_sector_count as u32 + fat_size * 2,
            current_cluster: 0,
            root_directory_cluster,
            use_cache,
            fat_cache: Cache::new(1, FAT16_CACHE_SIZE),
            cluster_cache: Cache::new(sector_per_cluster as usize, FAT16_CACHE_SIZE),
        })
    }

    pub fn format(
        device: &mut dyn BlockIO,
        start_addr: u32,
        volume_size: u32,
        use_cache: bool,
    ) -> Result<Self, ()> {
        let cluster_count =
            (volume_size - FAT16_RESERVED_SECTOR_COUNT) / FAT_SECTOR_PER_CLUSTER as u32;
        let fat_size = ceil(cluster_count, FAT16_CLUSTER_ENTRY_PER_SECTOR);
        let data_cluster_count = (volume_size - FAT16_RESERVED_SECTOR_COUNT - fat_size * 2)
            / FAT_SECTOR_PER_CLUSTER as u32;
        let fat_size = ceil(data_cluster_count, FAT16_CLUSTER_ENTRY_PER_SECTOR);

        let mut header = Box::new(FAT16Header::empty());
        header
            .common
            .jmp_boot_code
            .iter_mut()
            .zip([0xEB, 0x58, 0x90])
            .map(|(data, b)| *data = b);

        header
            .common
            .oem_id
            .iter_mut()
            .zip(b"MSDOS5.0")
            .map(|(data, &c)| *data = c);

        header.common.byte_per_sector = 512;
        header.common.sector_per_clustor = FAT_SECTOR_PER_CLUSTER;
        header.common.reserved_sector_count = 32;
        header.common.media_type = 0xF8;
        header.common.total_sector32 = volume_size;
        header.common.fat_size16 = fat_size as u16;

        let buffer = header.as_block();
        device
            .write(start_addr, slice::from_ref(&buffer))
            .map_err(|err| ())?;

        let fat_addr = start_addr + FAT16_RESERVED_SECTOR_COUNT;
        let fat2_addr = start_addr + FAT16_RESERVED_SECTOR_COUNT + fat_size;

        let mut fat_buffer: Vec<Block<512>> = vec![Block::empty()];
        *fat_buffer[0].get_mut::<u32>(0) = FAT16_END_OF_CLUSTER;
        device.write(fat_addr, &fat_buffer).map_err(|err| ())?;
        device.write(fat2_addr, &fat_buffer).map_err(|err| ())?;

        Ok(Self {
            start_addr,
            volume_size,
            reserved_sector_count: FAT16_RESERVED_SECTOR_COUNT,
            byte_per_sector: 512,
            sector_per_cluster: FAT_SECTOR_PER_CLUSTER,
            fat_addr,
            fat_size,
            fat2_addr,
            data_addr: start_addr + FAT16_RESERVED_SECTOR_COUNT + fat_size * 2,
            current_cluster: 0,
            root_directory_cluster: 0,
            use_cache,
            fat_cache: Cache::new(1, FAT16_CACHE_SIZE),
            cluster_cache: Cache::new(FAT_SECTOR_PER_CLUSTER as usize, FAT16_CACHE_SIZE),
        })
    }

    fn get_fat_sector(
        &mut self,
        device: &mut dyn BlockIO,
        offset: u32,
        buffer: &mut Vec<Block<512>>,
    ) -> Result<usize, usize> {
        let addr = self.fat_addr + offset;
        if self.use_cache {
            if let Ok(fat_data) = self.fat_cache.read_from_cache(addr as u64) {
                // buffer.clone_from(fat_data);
                return Ok(1);
            } else {
                let res = device.read(addr, buffer);
                self.fat_cache
                    .allocate_cache(addr as u64, &buffer, |address, buffer| {
                        device.write(address as u32, buffer);
                    });
                return res;
            }
        } else {
            device.read(addr as u32, buffer)
        }
    }

    fn set_fat_sector(
        &mut self,
        device: &mut dyn BlockIO,
        offset: u32,
        buffer: &Vec<Block<512>>,
    ) -> Result<usize, usize> {
        let addr = self.fat_addr + offset;
        if self.use_cache {
            if self.fat_cache.write_to_cache(addr as u64, &buffer).is_ok() {
                return Ok(1);
            } else {
                let res = device.write(addr, &buffer);
                self.fat_cache
                    .allocate_cache(addr as u64, &buffer, |address, buffer| {
                        device.write(address as u32, buffer);
                    });
                res
            }
        } else {
            device.write(addr, &buffer)
        }
    }

    fn get_free_cluster(&mut self, device: &mut dyn BlockIO) -> Result<u32, ()> {
        let free_cluster = self.current_cluster + 1;
        let cluster_entry_size = self.fat_size * FAT16_CLUSTER_ENTRY_PER_SECTOR;
        let mut buffer: Vec<Block<512>> = vec![Block::empty()];
        for cluster_offset in 1..=cluster_entry_size {
            let next_free_cluster = free_cluster.wrapping_add(cluster_offset);
            let cluster_sector = next_free_cluster / FAT16_CLUSTER_ENTRY_PER_SECTOR;
            let cluster_offset = next_free_cluster % FAT16_CLUSTER_ENTRY_PER_SECTOR;
            if cluster_offset == 0 || next_free_cluster % FAT16_CLUSTER_ENTRY_PER_SECTOR == 0 {
                self.get_fat_sector(device, cluster_sector, &mut buffer)
                    .map_err(|err| ())?;
            }
            if *buffer[0].get::<u16>(cluster_offset as usize) == 0 {
                self.current_cluster = next_free_cluster;
                return Ok(next_free_cluster);
            }
        }
        Err(())
    }

    fn get_cluster_ptr(&mut self, device: &mut dyn BlockIO, cluster: u32) -> Result<u32, ()> {
        let mut buffer: Vec<Block<512>> = vec![Block::empty()];
        let cluster_sector = cluster / FAT16_CLUSTER_ENTRY_PER_SECTOR;
        let cluster_offset = cluster % FAT16_CLUSTER_ENTRY_PER_SECTOR;
        if cluster_sector >= self.fat_size {
            return Err(());
        }
        device
            .read(self.fat_addr + cluster_offset, &mut buffer)
            .map_err(|err| ())?;
        Ok(*buffer[0].get::<u16>(cluster_offset as usize) as u32)
    }

    fn set_cluster_ptr(
        &mut self,
        device: &mut dyn BlockIO,
        cluster: u32,
        next_cluster: u32,
    ) -> Result<u32, ()> {
        let mut buffer: Vec<Block<512>> = vec![Block::empty()];
        let cluster_sector = cluster / FAT16_CLUSTER_ENTRY_PER_SECTOR;
        let cluster_offset = cluster % FAT16_CLUSTER_ENTRY_PER_SECTOR;
        if cluster_sector >= self.fat_size {
            return Err(());
        }
        device
            .read(self.fat_addr + cluster_offset, &mut buffer)
            .map_err(|err| ())?;
        *buffer[0].get_mut::<u16>(cluster_offset as usize) = next_cluster as u16;
        device
            .write(self.fat_addr + cluster_offset, &buffer)
            .map_err(|err| ())?;
        Ok(next_cluster)
    }

    fn read_cluster(
        &mut self,
        device: &mut dyn BlockIO,
        cluster: u32,
        buffer: &mut Vec<Block<512>>,
    ) -> Result<usize, usize> {
        let addr = self.data_addr + self.sector_per_cluster as u32 * cluster;
        if self.use_cache {
            if let Ok(cluster_data) = self.cluster_cache.read_from_cache(addr as u64) {
                buffer.clone_from(&cluster_data);
                return Ok(self.sector_per_cluster as usize);
            } else {
                let res = device.read(addr, buffer);
                self.cluster_cache
                    .allocate_cache(addr as u64, buffer, |address, buffer| {
                        device.write(address as u32, buffer);
                    });
                return res;
            }
        } else {
            device.read(addr, buffer)
        }
    }

    fn write_cluster(
        &mut self,
        device: &mut dyn BlockIO,
        cluster: u32,
        buffer: &Vec<Block<512>>,
    ) -> Result<usize, usize> {
        let addr = self.data_addr + self.sector_per_cluster as u32 * cluster;
        if self.use_cache {
            if self
                .cluster_cache
                .write_to_cache(addr as u64, buffer)
                .is_ok()
            {
                Ok(self.sector_per_cluster as usize)
            } else {
                let res = device.write(addr, buffer);
                self.cluster_cache
                    .allocate_cache(addr as u64, buffer, |address, buffer| {
                        device.write(address as u32, buffer);
                    });
                return res;
            }
        } else {
            device.write(addr, buffer)
        }
    }

    fn get_empty_dir_idx(&mut self, device: &mut dyn BlockIO, dir_cluster: u32) -> Result<u32, ()> {
        let entry_per_sector =
            self.sector_per_cluster as u32 * 512 / size_of::<DirectoryEntry>() as u32;
        let mut buffer: Vec<Block<512>> = vec![Block::empty(); self.sector_per_cluster as usize];
        self.read_cluster(device, dir_cluster, &mut buffer)
            .map_err(|err| ())?;
        for idx in 0..FAT_MAX_DIRECTORY_ENTRY_COUNT {
            let sector = idx / entry_per_sector;
            let sector_offset = idx % entry_per_sector;
            let entry = buffer[sector as usize].get::<DirectoryEntry>(sector_offset as usize);
            if entry.start_cluster_idx() == 0 {
                // debug!("entry_idx={idx}");
                return Ok(idx);
            }
        }
        Err(())
    }

    fn get_dir_entry(
        &mut self,
        device: &mut dyn BlockIO,
        dir_cluster: u32,
        dir_offset: u32,
    ) -> Result<DirectoryEntry, ()> {
        let entry_per_sector =
            self.sector_per_cluster as u32 * 512 / size_of::<DirectoryEntry>() as u32;
        let mut buffer: Vec<Block<512>> = vec![Block::empty(); self.sector_per_cluster as usize];
        self.read_cluster(device, dir_cluster, &mut buffer)
            .map_err(|err| ())?;
        Ok(buffer[(dir_offset / entry_per_sector) as usize]
            .get::<DirectoryEntry>((dir_offset % entry_per_sector) as usize)
            .clone())
    }

    fn set_dir_entry(
        &mut self,
        device: &mut dyn BlockIO,
        dir_cluster: u32,
        dir_offset: u32,
        data: &DirectoryEntry,
    ) -> Result<(), ()> {
        let entry_per_sector =
            self.sector_per_cluster as u32 * 512 / size_of::<DirectoryEntry>() as u32;
        let mut buffer: Vec<Block<512>> = vec![Block::empty(); self.sector_per_cluster as usize];
        self.read_cluster(device, dir_cluster, &mut buffer)
            .map_err(|err| ())?;
        // debug!("[FAT] dir_idx={dir_offset:X}, sector={sector:X}");
        *buffer[(dir_offset / entry_per_sector) as usize]
            .get_mut((dir_offset % entry_per_sector) as usize) = *data;
        self.write_cluster(device, dir_cluster, &buffer)
            .map_err(|err| ())?;
        Ok(())
    }

    const fn cluster_size(&self) -> u32 {
        self.byte_per_sector as u32 * self.sector_per_cluster as u32
    }
}

impl FileSystem for FAT16 {
    fn create(
        &mut self,
        device: &mut dyn BlockIO,
        dir: &DirectoryDescriptor,
        file_name: &str,
    ) -> Result<FileDescriptor, ()> {
        let dir_entry_idx = self.get_empty_dir_idx(device, dir.file_start_idx)?;
        let mut dir_entry = self.get_dir_entry(device, dir.file_start_idx, dir_entry_idx)?;
        let data_cluster = self.get_free_cluster(device)?;
        let file_name_byte = file_name.as_bytes();
        self.set_cluster_ptr(device, data_cluster, FAT16_END_OF_CLUSTER)?;
        for (idx, c) in dir_entry.name.iter_mut().enumerate() {
            if let Some(char) = file_name_byte.get(idx) {
                *c = *char;
            }
        }
        // debug!("Start Data Cluster={data_cluster:#X}");
        dir_entry.attr = FAT_DIR_ATTRIBUTE_FILE;
        dir_entry.set_start_cluster_idx(data_cluster);
        // debug!("[FAT] entry_idx={dir_entry_idx:#X}");
        self.set_dir_entry(device, dir.file_start_idx, dir_entry_idx, &dir_entry)?;

        Ok(FileDescriptor {
            file_start_idx: data_cluster,
            file_current_idx: data_cluster,
            dir_idx: dir.file_start_idx,
            dir_offset: dir_entry_idx,
            file_size: 0,
            ptr: 0,
        })
    }

    fn open(
        &mut self,
        device: &mut dyn BlockIO,
        dir: &DirectoryDescriptor,
        file_name: &str,
    ) -> Result<FileDescriptor, ()> {
        for offset in 0..FAT_MAX_DIRECTORY_ENTRY_COUNT {
            let entry = self.get_dir_entry(device, dir.file_start_idx, offset)?;
            if entry
                .name
                .iter()
                .zip(file_name.as_bytes())
                .all(|(c, nc)| *c == *nc)
            {
                if entry.attr != FAT_DIR_ATTRIBUTE_FILE {
                    return Err(());
                }
                let file_start_idx = entry.cluster_low as u32;
                // debug!("Start Data Cluster={file_start_idx:#X}");
                return Ok(FileDescriptor {
                    file_start_idx,
                    file_current_idx: file_start_idx,
                    dir_idx: dir.file_start_idx,
                    dir_offset: offset as u32,
                    file_size: entry.file_size,
                    ptr: 0,
                });
            }
        }
        Err(())
    }

    fn remove(&mut self, device: &mut dyn BlockIO, file: FileDescriptor) -> Result<(), ()> {
        let entry = DirectoryEntry::empty();
        let mut data_cluster = file.file_start_idx;
        self.set_dir_entry(device, file.dir_idx, file.dir_offset, &entry)?;

        while data_cluster < FAT16_END_OF_CLUSTER {
            let next_data_cluster = self.get_cluster_ptr(device, data_cluster)?;
            // debug!("remove: {data_cluster:#X} -> {next_data_cluster:#X}");
            self.set_cluster_ptr(device, data_cluster, 0)?;
            data_cluster = next_data_cluster;
        }
        Ok(())
    }

    fn read(
        &mut self,
        device: &mut dyn BlockIO,
        file: &mut FileDescriptor,
        buffer: &mut [u8],
    ) -> Result<usize, usize> {
        let byte_per_cluster = self.byte_per_sector as u32 * self.sector_per_cluster as u32;
        let mut dev_buffer: Vec<Block<512>> =
            vec![Block::empty(); self.sector_per_cluster as usize];
        let mut count = 0usize;
        self.read_cluster(device, file.file_current_idx, &mut dev_buffer)
            .map_err(|err| count)?;

        for data in buffer.iter_mut() {
            let file_cluster_offset = file.ptr % byte_per_cluster;
            *data = *dev_buffer[file_cluster_offset as usize / 512]
                .get_mut(file_cluster_offset as usize % 512);
            if file_cluster_offset == byte_per_cluster - 1 {
                file.file_current_idx = self
                    .get_cluster_ptr(device, file.file_current_idx)
                    .map_err(|err| count)?;
                if file.file_current_idx >= FAT16_END_OF_CLUSTER {
                    return Ok(count);
                }
                self.read_cluster(device, file.file_current_idx, &mut dev_buffer)
                    .map_err(|err| count)?;
            }
            file.ptr += 1;
            count += 1;
        }
        Ok(count)
    }

    fn write(
        &mut self,
        device: &mut dyn BlockIO,
        file: &mut FileDescriptor,
        buffer: &[u8],
    ) -> Result<usize, usize> {
        let byte_per_cluster = self.cluster_size();
        let mut dev_buffer: Vec<Block<512>> =
            vec![Block::empty(); self.sector_per_cluster as usize];
        let mut count = 0usize;
        self.read_cluster(device, file.file_current_idx, &mut dev_buffer)
            .map_err(|err| count)?;

        // debug!("[FAT] write data={buffer:?}");

        for &data in buffer.iter() {
            let file_cluster_offset = file.ptr % byte_per_cluster;
            *dev_buffer[file_cluster_offset as usize / 512]
                .get_mut(file_cluster_offset as usize % 512) = data;
            if file_cluster_offset == byte_per_cluster - 1 {
                self.write_cluster(device, file.file_current_idx, &dev_buffer)
                    .map_err(|err| count)?;
                let mut next_cluster = self
                    .get_cluster_ptr(device, file.file_current_idx)
                    .map_err(|err| count)?;
                if next_cluster >= FAT16_END_OF_CLUSTER {
                    let free_cluster = self.get_free_cluster(device).map_err(|err| count)?;
                    self.set_cluster_ptr(device, file.file_current_idx, free_cluster)
                        .map_err(|err| count)?;
                    self.set_cluster_ptr(device, free_cluster, FAT16_END_OF_CLUSTER)
                        .map_err(|err| count)?;
                    next_cluster = free_cluster;
                }
                self.read_cluster(device, next_cluster, &mut dev_buffer)
                    .map_err(|err| count)?;
                file.file_current_idx = next_cluster
            }
            file.ptr += 1;
            if file.ptr > file.file_size {
                file.file_size = file.ptr;
            }
            count += 1;
        }
        self.write_cluster(device, file.file_current_idx, &dev_buffer)
            .map_err(|err| count)?;
        let mut dir = self
            .get_dir_entry(device, file.dir_idx, file.dir_offset)
            .map_err(|err| count)?;
        dir.file_size += count as u32;
        self.set_dir_entry(device, file.dir_idx, file.dir_offset, &dir)
            .map_err(|err| count)?;
        Ok(count)
    }

    fn create_dir(
        &mut self,
        device: &mut dyn BlockIO,
        dir: &DirectoryDescriptor,
        dir_name: &str,
    ) -> Result<DirectoryDescriptor, ()> {
        let empty_dir_offset = self.get_empty_dir_idx(device, dir.file_start_idx)?;
        let mut dir_entry = self.get_dir_entry(device, dir.file_start_idx, empty_dir_offset)?;
        let free_cluster = self.get_free_cluster(device)?;

        dir_entry
            .name
            .iter_mut()
            .zip(dir_name.as_bytes().iter())
            .map(|(n, nc)| *n = *nc);
        dir_entry.attr = FAT_DIR_ATTRIBUTE_DIR;
        dir_entry.set_start_cluster_idx(free_cluster);
        self.set_dir_entry(device, dir.file_start_idx, empty_dir_offset, &dir_entry)?;

        self.set_cluster_ptr(device, free_cluster, FAT16_END_OF_CLUSTER)?;

        Ok(DirectoryDescriptor {
            file_start_idx: free_cluster,
            dir_idx: dir.file_start_idx,
            dir_offset: empty_dir_offset,
        })
    }

    fn open_dir(
        &mut self,
        device: &mut dyn BlockIO,
        dir: &DirectoryDescriptor,
        dir_name: &str,
    ) -> Result<DirectoryDescriptor, ()> {
        for offset in 0..FAT_MAX_DIRECTORY_ENTRY_COUNT {
            let entry = self.get_dir_entry(device, dir.file_start_idx, offset)?;
            if entry
                .name
                .iter()
                .zip(dir_name.as_bytes())
                .all(|(c, nc)| *c == *nc)
            {
                if entry.attr != FAT_DIR_ATTRIBUTE_DIR {
                    return Err(());
                }
                return Ok(DirectoryDescriptor {
                    file_start_idx: entry.start_cluster_idx(),
                    dir_idx: dir.file_start_idx,
                    dir_offset: offset,
                });
            }
        }
        Err(())
    }

    fn remove_dir(&mut self, device: &mut dyn BlockIO, dir: DirectoryDescriptor) -> Result<(), ()> {
        if dir.dir_idx == 0 {
            return Err(());
        }

        for offset in 0..FAT_MAX_DIRECTORY_ENTRY_COUNT {
            let entry = self.get_dir_entry(device, dir.file_start_idx, offset)?;
            let file_start_idx = entry.start_cluster_idx();
            if entry.attr == FAT_DIR_ATTRIBUTE_FILE {
                self.remove(
                    device,
                    FileDescriptor {
                        file_start_idx,
                        file_current_idx: file_start_idx,
                        dir_idx: dir.file_start_idx,
                        dir_offset: offset as u32,
                        file_size: entry.file_size,
                        ptr: 0,
                    },
                )?;
            } else if entry.attr == FAT_DIR_ATTRIBUTE_DIR {
                self.remove_dir(
                    device,
                    DirectoryDescriptor {
                        file_start_idx,
                        dir_idx: dir.file_start_idx,
                        dir_offset: offset as u32,
                    },
                )?;
            }
        }
        let entry = DirectoryEntry::empty();
        self.set_dir_entry(device, dir.dir_idx, dir.dir_offset, &entry)?;
        self.set_cluster_ptr(device, dir.file_start_idx, 0)?;

        Ok(())
    }

    fn root_dir(&mut self, device: &mut dyn BlockIO) -> Result<DirectoryDescriptor, ()> {
        Ok(DirectoryDescriptor {
            file_start_idx: self.root_directory_cluster,
            dir_idx: 0,
            dir_offset: 0,
        })
    }

    fn list_entry(
        &mut self,
        device: &mut dyn BlockIO,
        dir: &DirectoryDescriptor,
    ) -> Result<Vec<(usize, String)>, ()> {
        let mut list: Vec<(usize, String)> = Vec::new();
        for offset in 0..FAT_MAX_DIRECTORY_ENTRY_COUNT {
            let entry = self.get_dir_entry(device, dir.file_start_idx, offset)?;
            if entry.cluster_low != 0 {
                let name = AsciiStr::from_ascii(&entry.name)
                    .expect(&format!("name = {:?}", entry.name))
                    .to_string();
                list.push((offset as usize, name));
            }
        }
        Ok(list)
    }

    fn shrink(&mut self, device: &mut dyn BlockIO, file: &mut FileDescriptor) -> Result<(), ()> {
        let mut dir = self.get_dir_entry(device, file.dir_idx, file.dir_offset)?;
        dir.file_size = 0;
        self.set_dir_entry(device, file.dir_idx, file.dir_offset, &dir)?;
        file.file_size = 0;
        file.file_current_idx = file.file_start_idx;

        let mut data_cluster = file.file_start_idx;
        if data_cluster < FAT16_END_OF_CLUSTER {
            data_cluster = self.get_cluster_ptr(device, data_cluster)?;
            while data_cluster < FAT16_END_OF_CLUSTER {
                let next_data_cluster = self.get_cluster_ptr(device, data_cluster)?;
                // debug!("{data_cluster:#X} -> {next_data_cluster:#X}");
                self.set_cluster_ptr(device, data_cluster, 0)?;
                data_cluster = next_data_cluster;
            }
            self.set_cluster_ptr(device, file.file_start_idx, FAT16_END_OF_CLUSTER)?;
        }

        Ok(())
    }

    fn flush(&mut self, device: &mut dyn BlockIO) {
        if self.use_cache {
            self.fat_cache.flush(|address, buffer| {
                device.write(address as u32, buffer);
            });
            self.cluster_cache.flush(|address, buffer| {
                device.write(address as u32, buffer);
            })
        }
    }
}
