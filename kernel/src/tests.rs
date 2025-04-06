use kernel::{print, println};
use log::info;

fn print_input() {
    let mut count = 0usize;
    loop {
        print!("{}", getch() as char);
        count = count.wrapping_add(1);
        // if count % 10 == 0 {
        //     WindowFrame::new(200, 200, "Test");
        // }
    }
}

fn test_hdd() {
    let mut buffer: [Block<512>; 1] = [const { Block::empty() }; 1];
    let mut hdd = get_device(1).expect("Cannot find HDD");
    info!("PATA HDD Test Start");
    info!("1. Read");
    for lba in 0..4 {
        hdd.read_block(lba, &mut buffer).expect("HDD Read Failed");
        for (lba_offset, block) in buffer.iter().enumerate() {
            for idx in 0..512 {
                if idx % 16 == 0 {
                    print!(
                        "\nLBA={:2X}, offset={:3X}    |",
                        lba + lba_offset as u32,
                        idx
                    )
                }
                print!("{:02X} ", block.get::<u8>(idx));
            }
            println!();
        }
    }

    // info!("2. Write");
    // for block in buffer.iter_mut() {
    //     for idx in 0..512 {
    //         *block.get_mut(idx) = idx as u8;
    //     }
    // }
    // write_block(1, 0, &buffer).expect("HDD Write Failed");
    // read_block(1, 0, &mut buffer).expect("HDD Read Failed");
    // for (lba, block) in buffer.iter().enumerate() {
    //     for idx in 0..512 {
    //         if idx % 16 == 0 {
    //             print!("\nLBA={:2X}, offset={:3X}    |", lba, idx)
    //         }
    //         print!("{:02X} ", block.get::<u8>(idx));
    //     }
    //     println!();
    // }
}

fn test_hdd_rw() {
    let mut buffer: [Block<512>; 1] = [Block::empty(); 1];
    let mut pattern: [[Block<512>; 1]; 4] = [const { [Block::empty(); 1] }; 4];

    let hdd = get_device(1).expect("Cannot find HDD");

    for block in pattern[0].iter_mut() {
        for idx in 0..512 {
            *block.get_mut(idx) = idx as u8;
        }
    }

    for block in pattern[1].iter_mut() {
        for idx in 0..512 {
            *block.get_mut(idx) = (idx as u8) % 16;
        }
    }

    for block in pattern[2].iter_mut() {
        for idx in 0..512 {
            *block.get_mut(idx) = (idx as u8) % 2;
        }
    }

    for block in pattern[3].iter_mut() {
        for idx in 0..512 {
            if idx % 4 == 0 {
                *block.get_mut(idx) = 1;
            }
        }
    }

    let mut flag = false;
    info!("PATA HDD Read/Write Test Start");
    for (lba, pattern_buffer) in pattern.iter().enumerate() {
        info!("Pattern {}", lba + 1);
        hdd.write_block(lba as u32, pattern_buffer)
            .expect("HDD Write Failed");
        hdd.read_block(lba as u32, &mut buffer)
            .expect("HDD Read Failed");
        for idx in 0..512 {
            if *pattern_buffer[0].get::<u8>(idx) != *buffer[0].get::<u8>(idx) {
                flag = true;
                break;
            }
        }
        if flag {
            error!("Test Failed");
            for (pattern_block, block) in pattern_buffer.iter().zip(buffer.iter()) {
                for idx in 0..512 * 2 {
                    let offset = idx & 0x0F | (idx & !0x1F) >> 1;
                    if idx % 16 == 0 {
                        if idx % 32 == 0 {
                            print!("\nLBA={:2X}, offset={:3X}  |", lba as u32, idx)
                        } else {
                            print!(" |  ")
                        }
                    }
                    if (idx >> 4) & 0x01 == 0 {
                        print!("{:02X} ", block.get::<u8>(offset));
                    } else {
                        print!("{:02X} ", pattern_block.get::<u8>(offset));
                    }
                }
                println!();
            }
            return;
        }
    }
    info!("Test Success");
}

fn test_fs() {
    let dev_name = "ram0";

    let root = open_dir(dev_name, 0, "/", b"r").expect("Attempt to Open Root Directory Failed");
    let mut count = 0;
    for (idx, entry) in root.entries() {
        info!("[{idx}] /{entry}");
        count += 1;
    }
    info!("Total {count} entries");

    let mut buffer = [0u8; 11];
    if let Ok(mut file) = open(dev_name, 0, "/file", b"r") {
        info!("File Found");
        file.read(&mut buffer).expect("File Read Failed");
        info!("File data={buffer:?}");
        file.remove().expect("File Remove Failed");
        flush();
    } else {
        info!("File Not Found");
        let mut file = open(dev_name, 0, "/file", b"w").expect("File Create Failed");
        buffer = [4u8; 11];
        info!("File data={buffer:?}");
        file.write(&buffer).expect("File Write Failed");
        info!("File Write Complete");
        flush();
    }

    let root = open_dir(dev_name, 0, "/", b"r").expect("Attempt to Open Root Directory Failed");
    let mut count = 0;
    for (idx, entry) in root.entries() {
        info!("[{idx}] /{entry}");
        count += 1;
    }
    info!("Total {count} entries");
}

fn test() {
    for i in 0..10 {
        create_task(
            "test",
            TaskFlags::new().thread().set_priority(0).clone(),
            None,
            test_thread as u64,
            0,
            0,
        );
    }
    // for i in 0..50 {
    //     create_task(
    //         TaskFlags::new().thread().set_priority(130).clone(),
    //         None,
    //         test_thread as u64,
    //         0,
    //         0,
    //     );
    // }
    // for i in 0..50 {
    //     create_task(
    //         TaskFlags::new().thread().set_priority(200).clone(),
    //         None,
    //         test_thread as u64,
    //         0,
    //         0,
    //     );
    // }
    loop {
        schedule();
    }
}

fn test_fpu() {
    let id = running_task().unwrap().id() + 1;
    let mut count = 1.0f64;

    for i in 0..10 {
        let before = count;
        let factor = (id + i) as f64 / id as f64;
        count *= factor;
        // info!("PID={:3}| count(mul)={:.6}", id, count);
        count /= factor;
        // info!("PID={:3}| count(div)={:.6}", id, count);
        if before != count {
            info!(
                "PID={:3}| Test Failed, before={:.6}, after={:.6}",
                id, before, count
            );
            return;
        }
    }
    info!("PID={:3}| Test Success", id);
}

fn test_thread() {
    let id = running_task().unwrap().id() + 1;

    let mut random = id;
    let mut value1 = 1f64;
    let mut value2 = 1f64;

    let data = ["-", "\\", "|", "/"];
    let mut count = 0;

    let mut writer = create_window(8, 16);
    let mut fail = false;

    for _ in 0..1000 {
        random = random * 1103515245 + 12345;
        random = (random >> 16) & 0xFFFF_FFFF;
        let factor = random % 255;
        let factor = (factor + id) as f64 / id as f64;
        value1 *= factor;
        value2 *= factor;

        if value1 != value2 {
            fail = true;
            break;
        }

        value1 /= factor;
        value2 /= factor;

        if value1 != value2 {
            fail = true;
            break;
        }

        draw_str(
            Point(0, 0),
            data[count],
            PixelColor::Red,
            PixelColor::Black,
            &mut writer,
        );
        // render();
        count = (count + 1) % 4;
    }
    // draw_str(
    //     Point(0, 0),
    //     " ",
    //     PixelColor::Red,
    //     PixelColor::White,
    //     &mut writer,
    // );
    if fail {
        info!("Thread id={id}: FPU Test Failed -> left={value1}, right={value2}");
    } else {
        info!(
            "Thread id={}: Test End, Window id={}",
            id,
            writer.write_id().unwrap()
        );
    }
    writer.close();
}

fn test_windmill() {
    let id = running_task().unwrap().id() + 1;
    let data = [b'-', b'\\', b'|', b'/'];
    let offset = id * 2;
    let offset_x = id % 80 + 80;
    let offset_y = id / 80 + 25;
    let mut count = 0;

    loop {
        write_ascii(
            offset_x * 8,
            offset_y * 16,
            data[count],
            PixelColor::Red,
            Some(PixelColor::Black),
            &mut get_graphic().lock(),
        );
        count = (count + 1) % 4;
    }
}
