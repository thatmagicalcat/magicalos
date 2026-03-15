#!/bin/sh

PROJECT_ROOT=$(dirname "$0")
KERNEL_PATH=$1

echo "Building ISO Image with kernel: $KERNEL_PATH"

mkdir -p $PROJECT_ROOT/isodir/boot/grub
cp $KERNEL_PATH $PROJECT_ROOT/isodir/boot/kernel
cp $PROJECT_ROOT/grub.cfg $PROJECT_ROOT/isodir/boot/grub/

grub-mkrescue -o magical.iso $PROJECT_ROOT/isodir

echo "Launching QEMU"
qemu-system-x86_64 -m 2G -enable-kvm -cdrom magical.iso
