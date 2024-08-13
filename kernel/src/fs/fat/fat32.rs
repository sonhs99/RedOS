use core::str;
use core::{ptr::slice_from_raw_parts, slice};

use alloc::string::{String, ToString};
use alloc::vec;
use alloc::{boxed::Box, vec::Vec};
use ascii::{AsciiStr, AsciiString};
use log::debug;

use crate::device::hdd::{Block, BlockIO};

use super::{
    CommonFATHeader, DirectoryDescriptor, DirectoryEntry, FileDescriptor, FileSystem,
    FAT_DIR_ATTRIBUTE_DIR, FAT_DIR_ATTRIBUTE_FILE, FAT_END_OF_CLUSTER,
    FAT_MAX_DIRECTORY_ENTRY_COUNT, FAT_SECTOR_PER_CLUSTER, FAT_SECTOR_PER_CLUSTER_ENTRY,
};

const FAT32_RESERVED_SECTOR_COUNT: u32 = 32;
const FAT32_ROOT_DIRECTORY_CLUSTER: u32 = 2;

#[repr(C, packed)]
struct FAT32Header {
    common: CommonFATHeader,
    fat_size: u32,
    ext_flags: u16,
    fs_version: u16,
    root_directory_clustor: u32,
    fs_info: u16,
    boot_record_backup: u16,
    _reserved1: [u32; 3],
    drive_num: u8,
    _reserved2: u8,
    boot_signature: u8,
    volume_id: [u8; 4],
    volume_label: [u8; 11],
    fs_type: [u8; 8],
    _reserved3: [u8; 422],
}

impl FAT32Header {
    pub const fn empty() -> Self {
        Self {
            common: CommonFATHeader::empty(),
            fat_size: 0,
            ext_flags: 0,
            fs_version: 0,
            root_directory_clustor: 0,
            fs_info: 0,
            boot_record_backup: 0,
            _reserved1: [0; 3],
            drive_num: 0,
            _reserved2: 0,
            boot_signature: 0,
            volume_id: [0; 4],
            volume_label: [0; 11],
            fs_type: [0; 8],
            _reserved3: [0; 422],
        }
    }

    pub fn as_block(&self) -> &Block<512> {
        unsafe { &*(self as *const Self).cast::<Block<512>>() }
    }
}

#[repr(C, packed)]
struct FsInfo {
    lead_signature: u32,
    _reserved1: [u8; 480],
    struct_signature: u32,
    free_clustor_count: u32,
    next_free_clustor: u32,
    _reserved2: [u8; 12],
    trail_signature: u32,
}

impl FsInfo {
    pub const fn empty() -> Self {
        Self {
            lead_signature: 0,
            _reserved1: [0; 480],
            struct_signature: 0,
            free_clustor_count: 0,
            next_free_clustor: 0,
            _reserved2: [0; 12],
            trail_signature: 0,
        }
    }
    pub fn as_block(&self) -> &Block<512> {
        unsafe { &*(self as *const Self).cast::<Block<512>>() }
    }
}

pub struct FAT32 {
    start_addr: u32,
    volume_size: u32,
    reserved_sector_count: u32,

    byte_per_sector: u16,
    sector_per_cluster: u8,

    fat_addr: u32,
    fat2_addr: u32,
    fat_size: u32,

    data_addr: u32,

    free_cluster_count: u32,
    next_free_cluster: u32,
    root_directory_cluster: u32,
}

impl FAT32 {
    pub fn mount(device: &mut dyn BlockIO, start_addr: u32, volume_size: u32) -> Result<Self, ()> {
        let mut buffer = vec![Block::empty()];
        device.read(start_addr, &mut buffer).map_err(|err| ())?;
        let header = buffer[0].convert::<FAT32Header>();
        let byte_per_sector = header.common.byte_per_sector;
        let sector_per_cluster = header.common.sector_per_clustor;
        let reserved_sector_count = header.common.reserved_sector_count;

        let total_sector = header.common.total_sector32;
        let fat_size = header.fat_size;
        let root_directory_cluster = header.root_directory_clustor;

        debug!("Jmp Boot Code  : {:0X?}", header.common.jmp_boot_code);
        debug!("Byte/Sector    : {byte_per_sector}");
        debug!("Sector/Cluster : {sector_per_cluster}");
        debug!("Reserved Sector: {reserved_sector_count:#X}");
        debug!("Total Sector   : {total_sector:#X}");
        debug!("FAT Size       : {fat_size:#X}");
        debug!("Root Cluster   : {root_directory_cluster:#X}");

        device.read(start_addr + 1, &mut buffer).map_err(|err| ())?;
        let fs_info = buffer[0].convert::<FsInfo>();
        let free_cluster_count = fs_info.free_clustor_count;
        let next_free_cluster = fs_info.next_free_clustor;

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
            free_cluster_count,
            next_free_cluster,
            root_directory_cluster,
        })
    }

    pub fn format(device: &mut dyn BlockIO, start_addr: u32, volume_size: u32) -> Result<Self, ()> {
        let cluster_count = (volume_size - 32) / 8;
        let fat_size =
            (cluster_count + FAT_SECTOR_PER_CLUSTER_ENTRY - 1) / FAT_SECTOR_PER_CLUSTER_ENTRY;
        let data_cluster_count = (volume_size - 32 - fat_size * 2) / 8;
        let fat_size =
            (data_cluster_count + FAT_SECTOR_PER_CLUSTER_ENTRY - 1) / FAT_SECTOR_PER_CLUSTER_ENTRY;

        let mut header = Box::new(FAT32Header::empty());
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
        header.root_directory_clustor = 2;
        header.fat_size = fat_size;

        let buffer = header.as_block();
        device
            .write(start_addr, slice::from_ref(&buffer))
            .map_err(|err| ())?;

        let mut fs_info = FsInfo::empty();

        fs_info.free_clustor_count = data_cluster_count - 1;
        fs_info.next_free_clustor = 3;

        let buffer = fs_info.as_block();
        device
            .write(start_addr + 1, slice::from_ref(&buffer))
            .map_err(|err| ())?;

        let fat_addr = start_addr + FAT32_RESERVED_SECTOR_COUNT;
        let fat2_addr = start_addr + FAT32_RESERVED_SECTOR_COUNT + fat_size;

        let mut fat_buffer: Vec<Block<512>> = vec![Block::empty()];
        *fat_buffer[0].get_mut::<u32>(0) = FAT_END_OF_CLUSTER;
        *fat_buffer[0].get_mut::<u32>(1) = FAT_END_OF_CLUSTER;
        *fat_buffer[0].get_mut::<u32>(2) = FAT_END_OF_CLUSTER;
        device.write(fat_addr, &fat_buffer).map_err(|err| ())?;
        device.write(fat2_addr, &fat_buffer).map_err(|err| ())?;

        Ok(Self {
            start_addr,
            volume_size,
            reserved_sector_count: FAT32_RESERVED_SECTOR_COUNT,
            byte_per_sector: 512,
            sector_per_cluster: FAT_SECTOR_PER_CLUSTER,
            fat_addr,
            fat_size,
            fat2_addr,
            data_addr: start_addr + FAT32_RESERVED_SECTOR_COUNT + fat_size * 2,
            free_cluster_count: data_cluster_count - 1,
            next_free_cluster: 3,
            root_directory_cluster: 2,
        })
    }

    fn get_fat_sector(
        &mut self,
        device: &mut dyn BlockIO,
        offset: u32,
        buffer: &mut Block<512>,
    ) -> Result<usize, usize> {
        device.read(self.fat_addr + offset, slice::from_mut(buffer))
    }

    fn set_fat_sector(
        &mut self,
        device: &mut dyn BlockIO,
        offset: u32,
        buffer: &Block<512>,
    ) -> Result<usize, usize> {
        device.write(self.fat_addr + offset, slice::from_ref(buffer))
    }

    fn get_free_cluster(&mut self, device: &mut dyn BlockIO) -> Result<u32, ()> {
        let free_cluster = self.next_free_cluster;
        let cluster_entry_size = self.fat_size * FAT_SECTOR_PER_CLUSTER_ENTRY;
        let mut buffer: Block<512> = Block::empty();
        for cluster_offset in 1..=cluster_entry_size {
            let next_free_cluster = free_cluster.wrapping_add(cluster_offset);
            let cluster_sector = next_free_cluster / FAT_SECTOR_PER_CLUSTER_ENTRY;
            let cluster_offset = next_free_cluster % FAT_SECTOR_PER_CLUSTER_ENTRY;
            if cluster_offset == 0 || next_free_cluster % FAT_SECTOR_PER_CLUSTER_ENTRY == 0 {
                self.get_fat_sector(device, cluster_sector, &mut buffer)
                    .map_err(|err| ())?;
            }
            if *buffer.get::<u32>(cluster_offset as usize) == 0 {
                self.next_free_cluster = next_free_cluster;
                return Ok(free_cluster);
            }
        }
        Err(())
    }

    fn get_cluster_ptr(&mut self, device: &mut dyn BlockIO, cluster: u32) -> Result<u32, ()> {
        let mut buffer: Vec<Block<512>> = vec![Block::empty()];
        let cluster_sector = cluster / FAT_SECTOR_PER_CLUSTER_ENTRY;
        let cluster_offset = cluster % FAT_SECTOR_PER_CLUSTER_ENTRY;
        if cluster_sector >= self.fat_size {
            return Err(());
        }
        device
            .read(self.fat_addr + cluster_offset, &mut buffer)
            .map_err(|err| ())?;
        Ok(*buffer[0].get::<u32>(cluster_offset as usize))
    }

    fn set_cluster_ptr(
        &mut self,
        device: &mut dyn BlockIO,
        cluster: u32,
        next_cluster: u32,
    ) -> Result<u32, ()> {
        let mut buffer: Vec<Block<512>> = vec![Block::empty()];
        let cluster_sector = cluster / FAT_SECTOR_PER_CLUSTER_ENTRY;
        let cluster_offset = cluster % FAT_SECTOR_PER_CLUSTER_ENTRY;
        if cluster_sector >= self.fat_size {
            return Err(());
        }
        device
            .read(self.fat_addr + cluster_offset, &mut buffer)
            .map_err(|err| ())?;
        *buffer[0].get_mut::<u32>(cluster_offset as usize) = next_cluster;
        device
            .write(self.fat_addr + cluster_offset, &buffer)
            .map_err(|err| ())?;
        Ok(next_cluster)
    }

    fn read_cluster(
        &mut self,
        device: &mut dyn BlockIO,
        cluster: u32,
        buffer: &mut [Block<512>],
    ) -> Result<usize, usize> {
        let offset = self.sector_per_cluster as u32 * cluster;
        device.read(self.data_addr + offset, buffer)
    }

    fn write_cluster(
        &mut self,
        device: &mut dyn BlockIO,
        cluster: u32,
        buffer: &[Block<512>],
    ) -> Result<usize, usize> {
        let offset = self.sector_per_cluster as u32 * cluster;
        device.write(self.data_addr + offset, buffer)
    }

    fn get_empty_dir_idx(&mut self, device: &mut dyn BlockIO, dir_cluster: u32) -> Result<u32, ()> {
        let entry_per_sector = FAT_MAX_DIRECTORY_ENTRY_COUNT / self.sector_per_cluster as u32;
        let mut buffer: Vec<Block<512>> = vec![Block::empty(); 8];
        let offset = self.sector_per_cluster as u32 * dir_cluster;
        self.read_cluster(device, self.data_addr + offset, &mut buffer)
            .map_err(|err| ())?;
        for idx in 0..FAT_MAX_DIRECTORY_ENTRY_COUNT {
            let sector = idx / entry_per_sector;
            let sector_offset = idx % entry_per_sector;
            let entry = buffer[sector as usize].get::<DirectoryEntry>(sector_offset as usize);
            if entry.start_cluster_idx() == 0 {
                return Ok(idx);
            }
        }
        Err(())
    }

    fn get_dir_table(
        &mut self,
        device: &mut dyn BlockIO,
        dir_cluster: u32,
    ) -> Result<[DirectoryEntry; FAT_MAX_DIRECTORY_ENTRY_COUNT as usize], ()> {
        let entry_per_sector = FAT_MAX_DIRECTORY_ENTRY_COUNT / self.sector_per_cluster as u32;
        let mut buffer: Vec<Block<512>> = vec![Block::empty(); 8];
        let offset = self.sector_per_cluster as u32 * dir_cluster;
        let sector = dir_cluster / entry_per_sector;
        let sector_offset = dir_cluster % entry_per_sector;
        self.read_cluster(device, self.data_addr + offset, &mut buffer)
            .map_err(|err| ())?;
        unsafe {
            Ok(*buffer
                .as_ptr()
                .cast::<[DirectoryEntry; FAT_MAX_DIRECTORY_ENTRY_COUNT as usize]>())
        }
    }

    fn set_dir_data(
        &mut self,
        device: &mut dyn BlockIO,
        dir_cluster: u32,
        dir_idx: u32,
        data: &DirectoryEntry,
    ) -> Result<(), ()> {
        let entry_per_sector = FAT_MAX_DIRECTORY_ENTRY_COUNT / self.sector_per_cluster as u32;
        let mut buffer: Vec<Block<512>> = vec![Block::empty(); 8];
        let offset = self.sector_per_cluster as u32 * dir_cluster;
        let sector = dir_idx / entry_per_sector;
        let sector_offset = dir_idx % entry_per_sector;
        self.read_cluster(device, self.data_addr + offset, &mut buffer)
            .map_err(|err| ())?;
        *buffer[sector as usize].get_mut(sector_offset as usize) = *data;
        self.write_cluster(device, self.data_addr + offset, &buffer)
            .map_err(|err| ())?;
        Ok(())
    }

    const fn cluster_size(&self) -> u32 {
        self.byte_per_sector as u32 * self.sector_per_cluster as u32
    }
}

impl FileSystem for FAT32 {
    fn create(
        &mut self,
        device: &mut dyn BlockIO,
        dir: &DirectoryDescriptor,
        file_name: &str,
    ) -> Result<FileDescriptor, ()> {
        let dir_entry_idx = self.get_empty_dir_idx(device, dir.file_start_idx)?;
        let mut dir_entry =
            (self.get_dir_table(device, dir.file_start_idx)?)[dir_entry_idx as usize];
        let data_cluster = self.get_free_cluster(device)?;
        let file_name_byte = file_name.as_bytes();
        self.set_cluster_ptr(device, data_cluster, FAT_END_OF_CLUSTER)?;
        for (idx, c) in dir_entry.name.iter_mut().enumerate() {
            if let Some(char) = file_name_byte.get(idx) {
                *c = *char;
            }
        }
        debug!("Start Data Cluster={data_cluster:#X}");
        // dir_entry
        //     .name
        //     .iter_mut()
        //     .zip(file_name.as_bytes())
        //     .map(|(c, nc)| *c = *nc);
        dir_entry.attr = FAT_DIR_ATTRIBUTE_FILE;
        dir_entry.set_start_cluster_idx(data_cluster);

        self.set_dir_data(device, dir.file_start_idx, dir_entry_idx, &dir_entry)?;
        let test_entry = (self.get_dir_table(device, dir.file_start_idx)?)[dir_entry_idx as usize];
        debug!(
            "entry={:#X}, target={:#X}",
            test_entry.start_cluster_idx(),
            data_cluster
        );
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
        for (idx, entry) in self
            .get_dir_table(device, dir.file_start_idx)?
            .iter()
            .enumerate()
        {
            if entry
                .name
                .iter()
                .zip(file_name.as_bytes())
                .all(|(c, nc)| *c == *nc)
            {
                if entry.attr != FAT_DIR_ATTRIBUTE_FILE {
                    return Err(());
                }
                let file_start_idx = entry.start_cluster_idx();
                debug!("Start Data Cluster={file_start_idx:#X}");
                return Ok(FileDescriptor {
                    file_start_idx,
                    file_current_idx: file_start_idx,
                    dir_idx: dir.file_start_idx,
                    dir_offset: idx as u32,
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
        self.set_dir_data(device, file.dir_idx, file.dir_offset, &entry)?;

        while data_cluster != FAT_END_OF_CLUSTER {
            let next_data_cluster = self.get_cluster_ptr(device, data_cluster)?;
            debug!("{data_cluster:#X} -> {next_data_cluster:#X}");
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
        let mut dev_buffer: Vec<Block<512>> = vec![Block::empty(); 8];
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
                if file.file_current_idx == FAT_END_OF_CLUSTER {
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
        let mut dev_buffer: Vec<Block<512>> = vec![Block::empty(); 8];
        let mut count = 0usize;
        self.read_cluster(device, file.file_current_idx, &mut dev_buffer)
            .map_err(|err| count)?;

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
                if next_cluster == FAT_END_OF_CLUSTER {
                    let free_cluster = self.get_free_cluster(device).map_err(|err| count)?;
                    self.set_cluster_ptr(device, file.file_current_idx, free_cluster)
                        .map_err(|err| count)?;
                    self.set_cluster_ptr(device, free_cluster, FAT_END_OF_CLUSTER)
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
        let mut dir = self
            .get_dir_table(device, file.dir_idx)
            .map_err(|err| count)?[file.dir_offset as usize];
        dir.file_size += count as u32;
        self.set_dir_data(device, file.dir_idx, file.dir_offset, &dir)
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
        let mut dir_entry =
            self.get_dir_table(device, dir.file_start_idx)?[empty_dir_offset as usize];
        let free_cluster = self.get_free_cluster(device)?;

        dir_entry
            .name
            .iter_mut()
            .zip(dir_name.as_bytes().iter())
            .map(|(n, nc)| *n = *nc);
        dir_entry.attr = FAT_DIR_ATTRIBUTE_DIR;
        dir_entry.set_start_cluster_idx(free_cluster);
        self.set_dir_data(device, dir.file_start_idx, empty_dir_offset, &dir_entry)?;

        self.set_cluster_ptr(device, free_cluster, FAT_END_OF_CLUSTER)?;

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
        for (idx, entry) in self
            .get_dir_table(device, dir.file_start_idx)?
            .iter()
            .enumerate()
        {
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
                    dir_offset: idx as u32,
                });
            }
        }
        Err(())
    }

    fn remove_dir(&mut self, device: &mut dyn BlockIO, dir: DirectoryDescriptor) -> Result<(), ()> {
        if dir.dir_idx == 0 {
            return Err(());
        }

        let entry_table = self.get_dir_table(device, dir.file_start_idx)?;
        for (offset, entry) in entry_table.iter().enumerate() {
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
        self.set_dir_data(device, dir.dir_idx, dir.dir_offset, &entry)?;
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
        let entry_table = self.get_dir_table(device, dir.file_start_idx)?;
        for (offset, entry) in entry_table.iter().enumerate() {
            let file_start_idx = entry.start_cluster_idx();
            if file_start_idx != 0 {
                list.push((
                    offset,
                    AsciiStr::from_ascii(&entry.name).unwrap().to_string(),
                ));
            }
        }
        Ok(list)
    }

    fn shrink(&mut self, device: &mut dyn BlockIO, file: &mut FileDescriptor) -> Result<(), ()> {
        let mut dir = self.get_dir_table(device, file.dir_idx)?[file.dir_offset as usize];
        dir.file_size = 0;
        self.set_dir_data(device, file.dir_idx, file.dir_offset, &dir)?;
        file.file_size = 0;
        file.file_current_idx = file.file_start_idx;

        let mut data_cluster = file.file_start_idx;
        if data_cluster != FAT_END_OF_CLUSTER {
            data_cluster = self.get_cluster_ptr(device, data_cluster)?;
            while data_cluster != FAT_END_OF_CLUSTER {
                let next_data_cluster = self.get_cluster_ptr(device, data_cluster)?;
                debug!("{data_cluster:#X} -> {next_data_cluster:#X}");
                self.set_cluster_ptr(device, data_cluster, 0)?;
                data_cluster = next_data_cluster;
            }
            self.set_cluster_ptr(device, file.file_start_idx, FAT_END_OF_CLUSTER)?;
        }

        Ok(())
    }
}
