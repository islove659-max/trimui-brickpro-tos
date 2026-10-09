#!/bin/sh
# tos_gamemode.sh on|off|status  — "chế độ game": dừng dịch vụ nền không cần khi mở app/game,
# bật lại khi về menu. Gọi từ preload.sh (sau khi MainUI thoát, trước khi chạy game)
# và premainui.sh (trước khi MainUI chạy lại). Hoàn tác: GAMEMODE=0 trong tos.conf,
# hoặc xoá các file preload.sh/premainui.sh trong overlay.
CONF=/mnt/UDISK/tos/tos.conf
ST=/tmp/tos_gm
LOG=/tmp/tos_gm.log

GAMEMODE=1
GM_STOP="sftpgo MtpDaemon"        # tên tiến trình cần dừng khi vào game. KHÔNG thêm ntpd (tự nhân đôi khi chạy lại). musicserver mặc định GIỮ để còn phát nhạc nền
GM_DROP_CACHES=0                  # 1 = drop_caches khi vào game (thường không cần)
GM_REFRESH="trimui_osdd"        # tiến trình nền hay phình bộ nhớ: khởi động lại khi vào game nếu RssAnon > ngưỡng
GM_REFRESH_MIN_KB=30000
[ -f "$CONF" ] && . "$CONF"

log() { echo "$(date +%T) $*" >> "$LOG"; }
memav() { awk '/MemAvailable/{print $2}' /proc/meminfo; }
# getenv <pid> <TEN>: giá trị biến môi trường của tiến trình đang chạy
getenv() { tr '\0' '\n' < /proc/$1/environ 2>/dev/null | grep "^$2=" | head -1 | cut -d= -f2-; }

# Khởi động lại tiến trình đang phình (giữ nguyên dòng lệnh, thư mục và LD_LIBRARY_PATH)
refresh() {
    for n in $GM_REFRESH; do
        p=$(pgrep "$n" 2>/dev/null | head -1); [ -n "$p" ] || continue
        anon=$(awk '/^RssAnon:/{print $2}' /proc/$p/status 2>/dev/null)
        [ -n "$anon" ] && [ "$anon" -gt "$GM_REFRESH_MIN_KB" ] || continue
        cwd=$(readlink /proc/$p/cwd); cmd=$(tr '\0' ' ' < /proc/$p/cmdline)
        ldp=$(getenv $p LD_LIBRARY_PATH); pth=$(getenv $p PATH); hom=$(getenv $p HOME)
        kill -TERM "$p"; i=0
        while [ -d /proc/$p ] && [ $i -lt 10 ]; do i=$((i + 1)); sleep 0.1; done
        [ -d /proc/$p ] && kill -9 "$p"
        ( cd "${cwd:-/}"; [ -n "$ldp" ] && export LD_LIBRARY_PATH="$ldp"; [ -n "$pth" ] && export PATH="$pth"; [ -n "$hom" ] && export HOME="$hom"; sh -c "$cmd" </dev/null >/dev/null 2>&1 & )
        log "refresh $n: RssAnon ${anon} kB -> restart"
    done
}

case "$1" in
on)
    [ "$GAMEMODE" = 1 ] || exit 0
    [ -f "$ST/active" ] && exit 0
    mkdir -p "$ST"
    before=$(memav)
    for n in $GM_STOP; do
        for p in $(pgrep "$n" 2>/dev/null); do
            [ "$p" = "$$" ] && continue
            cmd=$(tr '\0' ' ' < /proc/$p/cmdline 2>/dev/null)
            [ -n "$cmd" ] || continue
            cwd=$(readlink /proc/$p/cwd 2>/dev/null)
            printf '%s\n%s\n%s\n%s\n%s\n' "${cwd:-/}" "$cmd" "$(getenv $p LD_LIBRARY_PATH)" "$(getenv $p PATH)" "$(getenv $p HOME)" > "$ST/$n.$p.cmd"
            kill -TERM "$p" 2>/dev/null
            log "stop $n pid=$p"
        done
    done
    i=0
    while [ $i -lt 10 ]; do
        alive=0
        for f in "$ST"/*.cmd; do
            [ -f "$f" ] || continue
            p=$(basename "$f" .cmd); p=${p##*.}
            [ -d /proc/$p ] && { kill -9 "$p" 2>/dev/null; alive=1; }
        done
        [ $alive = 0 ] && break
        i=$((i + 1)); sleep 0.1
    done
    refresh
    [ "$GM_DROP_CACHES" = 1 ] && { sync; echo 3 > /proc/sys/vm/drop_caches; }
    echo "$before" > "$ST/mem_before"; : > "$ST/active"
    log "ON  MemAvailable $before -> $(memav) kB"
    ;;
off)
    [ -f "$ST/active" ] || exit 0
    for f in "$ST"/*.cmd; do
        [ -f "$f" ] || continue
        cwd=$(sed -n 1p "$f"); cmd=$(sed -n 2p "$f"); ldp=$(sed -n 3p "$f"); pth=$(sed -n 4p "$f"); hom=$(sed -n 5p "$f")
        ( cd "$cwd" 2>/dev/null; [ -n "$ldp" ] && export LD_LIBRARY_PATH="$ldp"; [ -n "$pth" ] && export PATH="$pth"; [ -n "$hom" ] && export HOME="$hom"; sh -c "$cmd" </dev/null >/dev/null 2>&1 & )
        log "start: $cmd"
    done
    # Không dùng `find -delete`: busybox 1.27.2 trong rootfs gốc của Brick Pro không hỗ trợ (bản mới do thẻ SD/overlay đặt vào thì có).
    for f in "$ST"/*; do [ -f "$f" ] && rm -f "$f"; done
    log "OFF MemAvailable $(memav) kB"
    ;;
status)
    if [ -f "$ST/active" ]; then echo "ON (truoc: $(cat $ST/mem_before) kB, nay: $(memav) kB)"; else echo "OFF ($(memav) kB)"; fi
    ;;
*) echo "dung: $0 on|off|status"; exit 1 ;;
esac
