use core::slice;

use alloc::vec;
use alloc::{
    boxed::Box,
    string::{String, ToString},
    vec::Vec,
};
use fat::FAT32;
use hashbrown::HashMap;
use log::debug;

use crate::{
    device::block::{Block, BlockIO},
    sync::{Mutex, OnceLock},
};

pub mod fat;

trait FileSystem {
    fn create(
        &mut self,
        device: &mut dyn BlockIO,
        dir: &DirectoryDescriptor,
        file_name: &str,
    ) -> Result<FileDescriptor, ()>;

    fn open(
        &mut self,
        device: &mut dyn BlockIO,
        dir: &DirectoryDescriptor,
        file_name: &str,
    ) -> Result<FileDescriptor, ()>;

    fn read(
        &mut self,
        device: &mut dyn BlockIO,
        file: &mut FileDescriptor,
        buffer: &mut [u8],
    ) -> Result<usize, usize>;

    fn write(
        &mut self,
        device: &mut dyn BlockIO,
        file: &mut FileDescriptor,
        buffer: &[u8],
    ) -> Result<usize, usize>;

    fn remove(&mut self, device: &mut dyn BlockIO, file: FileDescriptor) -> Result<(), ()>;

    fn shrink(&mut self, device: &mut dyn BlockIO, file: &mut FileDescriptor) -> Result<(), ()>;

    fn create_dir(
        &mut self,
        device: &mut dyn BlockIO,
        dir: &DirectoryDescriptor,
        dir_name: &str,
    ) -> Result<DirectoryDescriptor, ()>;
    fn open_dir(
        &mut self,
        device: &mut dyn BlockIO,
        dir: &DirectoryDescriptor,
        dir_name: &str,
    ) -> Result<DirectoryDescriptor, ()>;

    fn list_entry(
        &mut self,
        device: &mut dyn BlockIO,
        dir: &DirectoryDescriptor,
    ) -> Result<Vec<(usize, String)>, ()>;

    fn remove_dir(&mut self, device: &mut dyn BlockIO, dir: DirectoryDescriptor) -> Result<(), ()>;
    fn root_dir(&mut self, device: &mut dyn BlockIO) -> Result<DirectoryDescriptor, ()>;

    fn flush(&mut self, device: &mut dyn BlockIO);
}

pub(crate) struct FileDescriptor {
    file_start_idx: u32,
    file_current_idx: u32,
    dir_idx: u32,
    dir_offset: u32,
    file_size: u32,
    ptr: u32,
}

pub(crate) struct DirectoryDescriptor {
    file_start_idx: u32,
    dir_idx: u32,
    dir_offset: u32,
}

pub struct File {
    file_desc: FileDescriptor,
    dev_name: String,
    port: u16,
}

impl File {
    pub fn remove(mut self) -> Result<(), ()> {
        ROOT_FS.lock().remove_file(self)
    }

    pub fn read(&mut self, buffer: &mut [u8]) -> Result<usize, usize> {
        ROOT_FS.lock().read_file(self, buffer)
    }

    pub fn write(&mut self, buffer: &[u8]) -> Result<usize, usize> {
        ROOT_FS.lock().write_file(self, buffer)
    }
}

pub struct Directory {
    dir_desc: DirectoryDescriptor,
    entry: Vec<(usize, String)>,
    dev_name: String,
    port: u16,
}

impl Directory {
    pub fn entries(&self) -> impl Iterator<Item = &(usize, String)> {
        self.entry.iter()
    }
}

// scheme://dev_name:0/file/to/path/name

struct FileSystemEntry {
    device: Mutex<Box<dyn BlockIO>>,
    file_systems: Vec<Box<dyn FileSystem>>,
}

pub struct RootFS {
    tree: HashMap<String, FileSystemEntry>,
}

impl RootFS {
    pub fn new() -> Self {
        Self {
            tree: HashMap::new(),
        }
    }

    pub fn mount(
        &mut self,
        mut device: impl BlockIO + 'static,
        dev_name: &str,
        use_cache: bool,
    ) -> Result<usize, ()> {
        let mut buffer: Vec<Block<512>> = vec![Block::empty()];
        device.read(0, &mut buffer).map_err(|err| ())?;
        let mbr = buffer[0].mbr();
        let mut count = 0;

        let mut file_systems: Vec<Box<dyn FileSystem>> = Vec::with_capacity(4);
        for part_idx in 0..4 {
            let partition = mbr.partition(part_idx);
            // debug!("{partition:#0X?}");
            if partition.type_() != 0 {
                let start_addr = partition.start_address();
                let volume_size = partition.size();
                let mut vbr: Vec<Block<512>> = vec![Block::empty()];
                device.read(0, &mut vbr).map_err(|err| ())?;
                match fat::fat_type(&vbr[0]) {
                    fat::FATType::FAT32 => {
                        if let Ok(fs) =
                            FAT32::mount(&mut device, start_addr, volume_size, use_cache)
                        {
                            file_systems.push(Box::new(fs));
                            count += 1;
                        }
                    }
                    _ => {}
                }
            }
        }
        self.tree.insert(
            dev_name.to_string(),
            FileSystemEntry {
                device: Mutex::new(Box::new(device)),
                file_systems,
            },
        );
        Ok(count)
    }

    pub fn format_by_name(
        &mut self,
        dev_name: &str,
        size: u32,
        use_cache: bool,
    ) -> Result<(), &'static str> {
        let entry = self.tree.get_mut(dev_name).ok_or("Device Not Found")?;
        let mut buffer: Block<512> = Block::empty();
        let start_address = 3;
        let max_size = entry.device.lock().max_addr() - start_address;
        let size = if size > max_size { max_size } else { size };
        entry
            .device
            .lock()
            .read(0, slice::from_mut(&mut buffer))
            .map_err(|err| "Read Failed")?;

        let mbr = buffer.mbr_mut();
        let mut partition = mbr.partition(0);
        partition.set_size(size);
        partition.set_start_address(start_address);
        partition.set_type_(0x80);
        mbr.set_partition(0, partition);
        entry.device.write(0, slice::from_ref(&buffer));

        let fs = FAT32::format(&mut entry.device, start_address, size, use_cache)
            .map_err(|err| "File System Format Failed")?;
        entry.file_systems.insert(0, Box::new(fs));
        Ok(())
    }

    pub fn device_iter(&self) -> impl Iterator<Item = &String> {
        self.tree.keys()
    }

    pub fn create_file(&mut self, dev_name: &str, port: u16, path: &str) -> Result<File, ()> {
        let entry = self.tree.get_mut(dev_name).ok_or(())?;
        let fs = entry.file_systems.get_mut(port as usize).ok_or(())?;
        let mut dir = fs.root_dir(&mut entry.device)?;
        let splited_path: Vec<&str> = path.split('/').collect();
        // debug!("path={:?}", splited_path);
        // loop {}
        let file_name = splited_path.last().ok_or(())?;
        for dir_name in splited_path.iter().skip(1).rev().skip(1).rev() {
            dir = fs.open_dir(&mut entry.device, &dir, dir_name)?;
        }
        let file = fs.create(&mut entry.device, &dir, file_name)?;
        Ok(File {
            file_desc: file,
            dev_name: dev_name.to_string(),
            port,
        })
    }

    pub fn open_file(&mut self, dev_name: &str, port: u16, path: &str) -> Result<File, ()> {
        let entry = self.tree.get_mut(dev_name).ok_or(())?;
        let fs = entry.file_systems.get_mut(port as usize).ok_or(())?;
        let mut dir = fs.root_dir(&mut entry.device)?;
        let splited_path: Vec<&str> = path[1..].split('/').collect();
        let file_name = splited_path.last().ok_or(())?;
        for dir_name in splited_path.iter().rev().skip(1).rev() {
            dir = fs.open_dir(&mut entry.device, &dir, dir_name)?;
        }
        let file = fs.open(&mut entry.device, &dir, file_name)?;
        Ok(File {
            file_desc: file,
            dev_name: dev_name.to_string(),
            port,
        })
    }

    pub fn remove_file(&mut self, file: File) -> Result<(), ()> {
        let entry = self.tree.get_mut(&file.dev_name).ok_or(())?;
        let fs = entry.file_systems.get_mut(file.port as usize).ok_or(())?;
        fs.remove(&mut entry.device, file.file_desc)?;
        Ok(())
    }

    pub fn shrink_file(&mut self, file: &mut File) -> Result<(), ()> {
        let entry = self.tree.get_mut(&file.dev_name).ok_or(())?;
        let fs = entry.file_systems.get_mut(file.port as usize).ok_or(())?;
        fs.shrink(&mut entry.device, &mut file.file_desc)?;
        Ok(())
    }

    pub fn create_dir(&mut self, dev_name: &str, port: u16, path: &str) -> Result<Directory, ()> {
        let entry = self.tree.get_mut(dev_name).ok_or(())?;
        let fs = entry.file_systems.get_mut(port as usize).ok_or(())?;
        let mut dir = fs.root_dir(&mut entry.device)?;
        let splited_path: Vec<&str> = path[1..].split('/').collect();
        let dir_name = splited_path.last().ok_or(())?;
        for dir_name in splited_path.iter().rev().skip(1).rev() {
            dir = fs.open_dir(&mut entry.device, &dir, dir_name)?;
        }
        let dir = fs.create_dir(&mut entry.device, &dir, dir_name)?;
        let entry_list = fs.list_entry(&mut entry.device, &dir)?;
        Ok(Directory {
            dir_desc: dir,
            entry: entry_list,
            dev_name: dev_name.to_string(),
            port,
        })
    }

    pub fn open_dir(&mut self, dev_name: &str, port: u16, path: &str) -> Result<Directory, ()> {
        let entry = self.tree.get_mut(dev_name).ok_or(())?;
        let fs = entry.file_systems.get_mut(port as usize).ok_or(())?;
        // debug!("path={path}");
        let mut dir = fs.root_dir(&mut entry.device)?;
        if path != "/" {
            let splited_path: Vec<&str> = path[1..].split('/').collect();
            for dir_name in splited_path.iter() {
                dir = fs.open_dir(&mut entry.device, &dir, dir_name)?;
            }
        }

        let entry_list = fs.list_entry(&mut entry.device, &dir)?;
        Ok(Directory {
            dir_desc: dir,
            entry: entry_list,
            dev_name: dev_name.to_string(),
            port,
        })
    }

    pub fn remove_dir(&mut self, dir: Directory) -> Result<(), ()> {
        let entry = self.tree.get_mut(&dir.dev_name).ok_or(())?;
        let fs = entry.file_systems.get_mut(dir.port as usize).ok_or(())?;
        fs.remove_dir(&mut entry.device, dir.dir_desc)?;
        Ok(())
    }

    pub fn read_file(&mut self, file: &mut File, buffer: &mut [u8]) -> Result<usize, usize> {
        let entry = self.tree.get_mut(&file.dev_name).ok_or(0usize)?;
        let fs = entry
            .file_systems
            .get_mut(file.port as usize)
            .ok_or(0usize)?;
        fs.read(&mut entry.device, &mut file.file_desc, buffer)
    }

    pub fn write_file(&mut self, file: &mut File, buffer: &[u8]) -> Result<usize, usize> {
        let entry = self.tree.get_mut(&file.dev_name).ok_or(0usize)?;
        let fs = entry
            .file_systems
            .get_mut(file.port as usize)
            .ok_or(0usize)?;
        fs.write(&mut entry.device, &mut file.file_desc, buffer)
    }

    pub fn flush(&mut self) {
        for entry in self.tree.values_mut() {
            for fs in entry.file_systems.iter_mut() {
                fs.flush(&mut entry.device);
            }
        }
    }
}

static ROOT_FS: OnceLock<Mutex<RootFS>> = OnceLock::new();

pub fn init_fs() {
    ROOT_FS.get_or_init(|| Mutex::new(RootFS::new()));
}

pub fn mount(device: impl BlockIO + 'static, dev_name: &str, use_cache: bool) -> Result<usize, ()> {
    ROOT_FS.lock().mount(device, dev_name, use_cache)
}

pub fn format_by_name(dev_name: &str, size: u32, use_cache: bool) -> Result<(), &'static str> {
    ROOT_FS.lock().format_by_name(dev_name, size, use_cache)
}

pub fn dev_list() -> Vec<String> {
    ROOT_FS
        .lock()
        .device_iter()
        .map(|name| name.clone())
        .collect()
}

pub fn open(dev_name: &str, port: u16, file_name: &str, mode: &[u8]) -> Result<File, &'static str> {
    let mut read = false;
    let mut write = false;
    let mut append = false;
    for c in mode {
        match c {
            b'r' | b'R' => read = true,
            b'w' | b'W' => write = true,
            b'a' | b'A' => append = true,
            _ => {}
        }
    }
    write = write || append;
    // debug!("read={read}, write={write}, append={append}");
    let mut search = ROOT_FS.lock().open_file(dev_name, port, file_name);
    // debug!("{}", search.is_ok());
    let mut file = match search {
        Ok(mut file) => {
            if write {
                ROOT_FS
                    .lock()
                    .shrink_file(&mut file)
                    .map_err(|err| "File Shrink Failed")?;
                // debug!("File has been Shrinked");
            }
            file
        }
        Err(_) => {
            if write {
                ROOT_FS
                    .lock()
                    .create_file(dev_name, port, file_name)
                    .map_err(|err| "File Creation Failed")?
            } else {
                return Err("Cannot Find File");
            }
        }
    };
    if append {
        file.file_desc.ptr = file.file_desc.file_size;
    }
    Ok(file)
}

pub fn open_dir(
    dev_name: &str,
    port: u16,
    dir_name: &str,
    mode: &[u8],
) -> Result<Directory, &'static str> {
    let mut read = false;
    let mut write = false;
    let mut append = false;
    for c in mode {
        match c {
            b'r' | b'R' => read = true,
            b'w' | b'W' => write = true,
            b'a' | b'A' => append = true,
            _ => {}
        }
    }
    write = write || append;
    // debug!("read={read}, write={write}, append={append}");
    let mut search = ROOT_FS.lock().open_dir(dev_name, port, dir_name);
    let mut dir = match search {
        Ok(dir) => dir,
        Err(_) => {
            if write {
                ROOT_FS
                    .lock()
                    .create_dir(dev_name, port, dir_name)
                    .map_err(|err| "Directory Create Failed")?
            } else {
                return Err("Cannot Find Directory");
            }
        }
    };
    Ok(dir)
}

pub fn flush() {
    ROOT_FS.lock().flush();
}
