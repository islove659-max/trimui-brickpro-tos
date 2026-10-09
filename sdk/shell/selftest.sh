#!/bin/sh
# Kiểm thử tos_env.sh: phát hiện môi trường + CPU lưu/đặt/khôi phục (không đụng MainUI).
D=$(cd "$(dirname "$0")" && pwd)
export TOS_STATE=/tmp/tos_selftest TOS_LOG=/tmp/tos_selftest.log
: > "$TOS_LOG"
. "$D/tos_env.sh"
tos_detect
echo "OS=$TOS_OS DEVICE=$TOS_DEVICE FB=${TOS_W}x${TOS_H}"
C=/sys/devices/system/cpu/cpu0/cpufreq
before=$(cat $C/scaling_governor)
tos_cpu_profile performance;  now=$(cat $C/scaling_governor)
tos_cpu_restore;              after=$(cat $C/scaling_governor)
echo "governor: truoc=$before  perf=$now  sau-khoi-phuc=$after"
[ "$before" = "$after" ] && echo "CPU RESTORE: OK" || echo "CPU RESTORE: LOI"
# Kiem thu tos_run voi app gia (ma thoat 3), khong dung MainUI vi gia lap OS=generic
TOS_OS=generic
tos_run balanced sh -c 'exit 3'; echo "tos_run rc=$?"
echo "governor sau tos_run: $(cat $C/scaling_governor)"
cat "$TOS_LOG"
