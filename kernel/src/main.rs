#![no_std]
#![no_main]

use core::arch::asm;

use bootloader::{BootInfo, FrameBufferConfig, PixelFormat};
use kernel::{
    console::{init_console, Console},
    device::pci::{init_pci, Pci, PciDevice},
    entry_point,
    font::write_ascii,
    graphic::{graphic, GraphicWriter, PixelColor},
    println,
};
use log::{info, warn};

entry_point!(kernel_main);

fn swithc_ehci_to_xhci(xhc_dev: &PciDevice) {
    let intel_ehc_exist = init_pci()
        .lock()
        .device_iter()
        .iter()
        .any(|&x| x.class_code.is_class(0x0c, 0x03, 0x20));
    if intel_ehc_exist {
        let superspeed_port = Pci::read_config(&xhc_dev, 0xdc);
        Pci::write_config(&xhc_dev, 0xd8, superspeed_port);
        let ehci2xhci_ports = Pci::read_config(&xhc_dev, 0xd4);
        Pci::write_config(&xhc_dev, 0xd0, ehci2xhci_ports);
        println!(
            "[ INFO ] swithc_ehci_to_xhci: SS = {superspeed_port:02X}, xHCI = {ehci2xhci_ports:02X}"
        )
    }
}

fn kernel_main(boot_info: BootInfo) {
    let (height, width) = boot_info.frame_config.resolution();

    let pixel_writer = graphic(boot_info.frame_config);
    pixel_writer.clean();

    init_console(pixel_writer, PixelColor::Black, PixelColor::White);

    let pci = init_pci();
    for dev in pci.lock().device_iter() {
        let vendor_id = dev.read_vendor_id();
        let class_code = dev.class_code;
        info!(
            "{}.{}.{}: vend {:04X}, class {:02X}{:02X}, head {:02x}",
            dev.bus, dev.dev, dev.func, vendor_id, class_code.base, class_code.sub, dev.header_type
        );
    }

    let mut xhc_dev: Option<PciDevice> = None;
    for &dev in pci.lock().device_iter() {
        if dev.class_code.is_class(0x0c, 0x03, 0x30) {
            xhc_dev = Some(dev);
            if dev.read_vendor_id() == 0x8086 {
                break;
            }
        }
    }

    if let Some(dev) = xhc_dev {
        info!("xHC has been found: {}.{}.{}", dev.bus, dev.dev, dev.func);
        let xhc_bar = dev.read_bar(0);
        info!("Read BAR0: 0x{xhc_bar:016X}");
        let xhc_mmio_base = xhc_bar & (!0xfu64);
        info!("xHC mmio base: 0x{xhc_mmio_base:016X}");

        if dev.read_vendor_id() == 0x8086 {
            swithc_ehci_to_xhci(&dev);
        }
    } else {
        warn!("no xHC Devices found");
    }
}
