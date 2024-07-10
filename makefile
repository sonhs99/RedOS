build:
	cargo -C ./kernel build --target x86_64.json --target-dir ../target -Z unstable-options
	cargo -C ./bootloader build --target x86_64-unknown-uefi --target-dir ../target -Z unstable-options
	cp ./target/x86_64/debug/kernel ./esp/kernel.elf
	cp ./target/x86_64-unknown-uefi/debug/bootloader.efi ./esp/efi/boot/bootx64.efi

run: build
	qemu-system-x86_64 \
    -m 512M \
    -drive if=pflash,format=raw,readonly=on,file=OVMF_CODE.fd \
    -drive if=pflash,format=raw,readonly=on,file=OVMF_VARS.fd \
    -drive format=raw,file=fat:rw:esp \
    -device qemu-xhci \
    -device usb-kbd \
    -device usb-mouse \
    -monitor stdio

run-without-usb: build
	qemu-system-x86_64 \
    -m 512M \
    -drive if=pflash,format=raw,readonly=on,file=OVMF_CODE.fd \
    -drive if=pflash,format=raw,readonly=on,file=OVMF_VARS.fd \
    -drive format=raw,file=fat:rw:esp \
    -monitor stdio

dump:
	objdump -d ./target/x86_64/debug/kernel > dump.txt

clean:
	rm -rf target