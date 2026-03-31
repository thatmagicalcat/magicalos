#!/bin/sh

PROJECT_ROOT=$(dirname "$0")
KERNEL_PATH=$1

DISK_IMG="$PROJECT_ROOT/build/magical_disk.img"
if [ ! -f "$DISK_IMG" ]; then
    echo "Creating empty disk image for storage testing..."
    dd if=/dev/zero of="$DISK_IMG" bs=1M count=64 status=none
fi

echo "Building ISO Image with kernel: $KERNEL_PATH"

mkdir -p $PROJECT_ROOT/build/isodir/boot/grub
cp $KERNEL_PATH $PROJECT_ROOT/build/isodir/boot/kernel
cp $PROJECT_ROOT/grub.cfg $PROJECT_ROOT/build/isodir/boot/grub/

grub-mkrescue -o $PROJECT_ROOT/build/magical.iso $PROJECT_ROOT/build/isodir 2> /dev/null

echo "Launching QEMU..."
qemu-system-x86_64                                     \
    -m 2G                                              \
    -enable-kvm                                        \
    -debugcon stdio                                    \
    -cpu host                                          \
    -display sdl                                       \
    -cdrom $PROJECT_ROOT/build/magical.iso             \
    -device ahci,id=ahci0                              \
    -drive id=disk0,file=$DISK_IMG,format=raw,if=none  \
    -device ide-hd,drive=disk0,bus=ahci0.0
