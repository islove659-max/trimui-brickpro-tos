#!/bin/sh
mkdir -p /tmp; mount -t proc p /proc; mount -t sysfs s /sys; mount -t devtmpfs d /dev
echo "=== virtio devices truoc khi nap driver"; for d in /sys/bus/virtio/devices/*; do echo "$d id=$(cat $d/device 2>/dev/null) driver=$(readlink $d/driver 2>/dev/null)"; done
modprobe virtio_mmio 2>&1|head -1; sleep 1
echo "=== virtio devices sau virtio_mmio"; for d in /sys/bus/virtio/devices/*; do echo "$d id=$(cat $d/device 2>/dev/null) driver=$(readlink $d/driver 2>/dev/null)"; done
insmod /mods/evdev.ko 2>&1|head -2; modprobe virtio_input 2>&1|head -2; sleep 1
echo "=== sau virtio_input"; for d in /sys/bus/virtio/devices/*; do echo "$d id=$(cat $d/device 2>/dev/null) driver=$(readlink $d/driver 2>/dev/null)"; done
ls /sys/class/input 2>&1; ls /dev/input 2>&1; cat /proc/bus/input/devices 2>&1 | head -5
dmesg | grep -iE "input|virtio" | tail -8
cat /proc/devices | grep -i input
echo VMTEST_DONE