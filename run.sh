#!/bin/sh

PROJECT_ROOT=$(dirname "$0")
KERNEL_PATH=$1
ISO_ROOT=$PROJECT_ROOT/build/isodir
LIMINE=$PROJECT_ROOT/limine/Limine/bin

echo "Building ISO Image with kernel: $KERNEL_PATH"

mkdir -p $ISO_ROOT/boot/limine

# cp -v $PROJECT_ROOT/ter-u32n.psf $ISO_ROOT/boot/limine/
cp -v $PROJECT_ROOT/wallpaper.png $ISO_ROOT/boot/limine/
cp -v $PROJECT_ROOT/limine.conf $ISO_ROOT/boot/limine/
cp -v $KERNEL_PATH $ISO_ROOT/boot/kernel
cp -v $LIMINE/limine-bios.sys $LIMINE/limine-bios-cd.bin $LIMINE/limine-uefi-cd.bin $ISO_ROOT/boot/limine

# Create the EFI boot tree and copy Limine's EFI executables over.
mkdir -p $ISO_ROOT/EFI/BOOT
cp -v $LIMINE/BOOTX64.EFI $ISO_ROOT/EFI/BOOT/

# Create the bootable ISO.
xorriso -as mkisofs -R -r -J -b boot/limine/limine-bios-cd.bin \
        -no-emul-boot -boot-load-size 4 -boot-info-table -hfsplus \
        -apm-block-size 2048 --efi-boot boot/limine/limine-uefi-cd.bin \
        -efi-boot-part --efi-boot-image --protective-msdos-label \
        $ISO_ROOT -o $PROJECT_ROOT/build/magical.iso

$LIMINE/limine bios-install $PROJECT_ROOT/build/magical.iso

echo "Launching QEMU..."
qemu-system-x86_64                                     \
    -no-reboot                                         \
    -cdrom $PROJECT_ROOT/build/magical.iso             \
    -m 2G                                              \
    -vga virtio                                        \
    -enable-kvm                                        \
    -debugcon stdio                                    \
    -cpu host                                          \
    -display gtk,zoom-to-fit=off                       
    # -device ahci,id=ahci0                              \
    # -drive id=disk0,file=$DISK_IMG,format=raw,if=none  \
    # -device ide-hd,drive=disk0,bus=ahci0.0
