build:
	cargo -C ./kernel build --target x86_64.json --target-dir ../target -Z unstable-options
	cargo -C ./bootloader build --target x86_64-unknown-uefi --target-dir ../target -Z unstable-options
	cp ./target/x86_64/debug/kernel ./esp/kernel.elf
	cp ./target/x86_64-unknown-uefi/debug/bootloader.efi ./esp/efi/boot/bootx64.efi

run: build
	qemu-system-x86_64 \
    -drive if=pflash,format=raw,readonly=on,file=OVMF_CODE.fd \
    -drive if=pflash,format=raw,readonly=on,file=OVMF_VARS.fd \
    -drive format=raw,file=fat:rw:esp \
    -usb \
    -device virtio-tablet \
    -device virtio-keyboard \
    -device qemu-xhci \
	-device usb-mouse \
    -monitor stdio

clean:
	rm -rf target