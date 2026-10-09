#!/bin/sh
# tos_env.sh — lớp nền dùng chung cho launcher app (Stock / Knulli / Spruce / NextUI).
# POSIX sh + busybox ash. Dùng: `. "$(dirname "$0")/tos_env.sh"; tos_detect; ...`
# Quy ước: không ghi ra ngoài /tmp trừ khi nói rõ; mọi thay đổi hệ thống đều có khôi phục.

TOS_STATE=${TOS_STATE:-/tmp/tos_state}
TOS_LOG=${TOS_LOG:-/tmp/tos.log}

tos_log() { echo "[$(date +%H:%M:%S)] $*" >> "$TOS_LOG"; }

# --- Phát hiện môi trường -> TOS_OS, TOS_DEVICE, TOS_W, TOS_H -------------------------
tos_detect() {
    if [ -n "$PLATFORM" ] || [ -f /mnt/SDCARD/MinUI.zip ]; then TOS_OS=nextui
    elif [ -d /userdata/system ] || [ -f /etc/batocera-version ] || [ -f /etc/knulli-version ]; then TOS_OS=knulli
    elif [ -d /mnt/SDCARD/spruce ] || [ -f /mnt/SDCARD/spruce/spruce ]; then TOS_OS=spruce
    elif [ -x /usr/trimui/bin/MainUI ] || [ -f /usr/trimui/bin/runtrimui.sh ]; then TOS_OS=stock
    else TOS_OS=generic; fi

    # Màn hình: đọc từ fb, KHÔNG đoán theo tên máy.
    TOS_W=0; TOS_H=0
    m=$(cat /sys/class/graphics/fb0/modes 2>/dev/null | head -1 | sed -r 's/^[A-Za-z]:([0-9]+)x([0-9]+).*/\1 \2/')
    case "$m" in
        [0-9]*" "[0-9]*) TOS_W=${m% *}; TOS_H=${m#* } ;;
    esac
    if [ "$TOS_W" = 0 ] && [ -r /sys/class/graphics/fb0/virtual_size ]; then
        v=$(cat /sys/class/graphics/fb0/virtual_size); TOS_W=${v%,*}
    fi
    case "${TOS_W}x${TOS_H}" in
        1024x768)  TOS_DEVICE=brick ;;
        1280x720)  TOS_DEVICE=smartpro ;;
        *)         TOS_DEVICE=unknown ;;
    esac
    export TOS_OS TOS_DEVICE TOS_W TOS_H
    tos_log "detect os=$TOS_OS device=$TOS_DEVICE fb=${TOS_W}x${TOS_H}"
}

# --- CPU: lưu trạng thái cũ, đặt profile, khôi phục đúng như cũ -----------------------
# Profile: performance | balanced | powersave. Không ép về 'ondemand' cứng.
tos_cpu_save() {
    mkdir -p "$TOS_STATE"
    C=/sys/devices/system/cpu/cpu0/cpufreq
    [ -f "$TOS_STATE/cpu.saved" ] && return 0
    {
        echo "gov=$(cat $C/scaling_governor 2>/dev/null)"
        echo "min=$(cat $C/scaling_min_freq 2>/dev/null)"
        echo "max=$(cat $C/scaling_max_freq 2>/dev/null)"
    } > "$TOS_STATE/cpu.saved"
}
tos_cpu_profile() {
    tos_cpu_save
    C=/sys/devices/system/cpu/cpu0/cpufreq
    case "$1" in
        performance) g=performance ;;
        powersave)   g=powersave ;;
        *)           g=ondemand ;;
    esac
    avail=$(cat $C/scaling_available_governors 2>/dev/null)
    case " $avail " in *" $g "*) echo $g > $C/scaling_governor ;; esac
    tos_log "cpu profile=$1 gov=$(cat $C/scaling_governor)"
}
tos_cpu_restore() {
    [ -f "$TOS_STATE/cpu.saved" ] || return 0
    C=/sys/devices/system/cpu/cpu0/cpufreq
    . "$TOS_STATE/cpu.saved"
    [ -n "$max" ] && echo "$max" > $C/scaling_max_freq 2>/dev/null
    [ -n "$min" ] && echo "$min" > $C/scaling_min_freq 2>/dev/null
    [ -n "$gov" ] && echo "$gov" > $C/scaling_governor 2>/dev/null
    rm -f "$TOS_STATE/cpu.saved"
    tos_log "cpu restored"
}

# --- Giao diện hệ: chỉ Stock mới cần dừng MainUI ---------------------------------------
tos_ui_pause()  { [ "$TOS_OS" = stock ] && killall -STOP MainUI 2>/dev/null; echo 0,0 > /sys/class/graphics/fb0/pan 2>/dev/null; return 0; }
tos_ui_resume() { [ "$TOS_OS" = stock ] && killall -CONT MainUI 2>/dev/null; return 0; }

# --- Chạy app có giám sát -------------------------------------------------------------
# tos_run <profile> <lệnh...>   Mã thoát 42 = OTA (chạy lại), 10 = tiếp tục nền (bỏ qua dọn dẹp).
# Trap đặt TRƯỚC khi dừng MainUI; app chạy nền + wait để tín hiệu tới được app.
tos_cleanup() {
    [ -n "$TOS_CLEANED" ] && return; TOS_CLEANED=1
    [ -n "$TOS_APP_PID" ] && kill -TERM "$TOS_APP_PID" 2>/dev/null
    tos_cpu_restore
    tos_ui_resume
    echo 0,0 > /sys/class/graphics/fb0/pan 2>/dev/null
    tos_log "cleanup done"
}
tos_run() {
    prof=$1; shift
    trap 'exit 143' TERM; trap 'exit 130' INT; trap tos_cleanup EXIT
    tos_ui_pause
    tos_cpu_profile "$prof"
    while :; do
        "$@" >> "$TOS_LOG" 2>&1 &
        TOS_APP_PID=$!
        wait $TOS_APP_PID; rc=$?
        TOS_APP_PID=
        tos_log "app exit rc=$rc"
        [ "$rc" = 42 ] && continue
        return $rc
    done
}
