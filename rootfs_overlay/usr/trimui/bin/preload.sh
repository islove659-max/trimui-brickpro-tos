#!/bin/sh

echo ondemand > /sys/devices/system/cpu/cpu0/cpufreq/scaling_governor
echo 408000 > /sys/devices/system/cpu/cpu0/cpufreq/scaling_min_freq
echo 2000000 > /sys/devices/system/cpu/cpu0/cpufreq/scaling_max_freq

rm /tmp/stay_alive
rm /tmp/stay_awake

# [tos] chạy sau khi MainUI thoát, trước khi game/app chạy: chỉ bật chế độ game khi thật sự sắp mở app
[ -f /tmp/cmd_to_run.sh ] && [ -x /usr/trimui/bin/tos_gamemode.sh ] && /usr/trimui/bin/tos_gamemode.sh on
