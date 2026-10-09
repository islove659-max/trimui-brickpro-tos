#!/bin/sh
# TOS 4.0 - bo cai dat tren may (Stock OS TrimUI Brick Pro). Dung:  install.sh install | uninstall | status
#   install    : cai runtime ARMHF 32-bit + glremote (GPU/am thanh cho game 32-bit) + toi uu he thong (che do game, udev, sysctl)
#   uninstall  : go sach, tra lai file goc
#   status     : cho biet da cai chua
# Chay duoc nhieu lan (cap nhat). Thay doi nam o tang ghi (overlay) cua he thong, KHONG dung toi bootloader/nhan/boot, khong xoa du lieu.
# Chi dung lenh co trong busybox goc cua may (khong timeout, khong sleep le).
BASE="$(cd "$(dirname "$0")" && pwd)"
PAY="$BASE/payload"
MANIFEST="$PAY/MANIFEST"
VERSION_FILE="$PAY/VERSION"
LOG="$BASE/install.log"
BK=/mnt/UDISK/tos/backup
STATE=/usr/lib/tos/installed
SHOW=/mnt/SDCARD/Apps/PortMaster/PortMaster/sdl2imgshow.aarch64
RES=/mnt/SDCARD/System/resources
BEGIN='# [tos-begin]'
END='# [tos-end]'

ver() { cat "$VERSION_FILE" 2>/dev/null | head -1; }
log() { echo "$(date '+%F %T') $*" >> "$LOG"; echo "$*"; }
msg() {   # msg "van ban" [giay]: in log + hien tren man hinh (neu co sdl2imgshow cua PortMaster)
    log "$1"
    if [ "${TOS_SCREEN:-1}" = 1 ] && [ -x "$SHOW" ] && [ -f "$RES/background.png" ]; then
        "$SHOW" -i "$RES/background.png" -f "$RES/DejaVuSans.ttf" -s 34 -c "0,0,0" -t "$1" >/dev/null 2>&1 &
        _p=$!
        sleep "${2:-2}"
        kill "$_p" 2>/dev/null
    fi
}
die() { msg "LOI: $1" 6; exit 1; }

check_env() {
    [ "$(uname -m)" = aarch64 ] || die "may khong phai aarch64"
    [ -f /usr/trimui/bin/preload.sh ] && [ -f /usr/trimui/bin/premainui.sh ] || die "khong phai Stock OS TrimUI (thieu preload.sh/premainui.sh)"
    [ -f "$MANIFEST" ] || die "thieu payload (giai nen day du goi vao goc the SD)"
}

verify_payload() {
    n=0
    while read -r t a b c d e; do
        [ "$t" = F ] || continue
        # F mode sha256 size relpath dest
        [ -f "$PAY/$d" ] || die "thieu tep goi: $d"
        h=$(sha256sum "$PAY/$d" | cut -d' ' -f1)
        [ "$h" = "$b" ] || die "tep goi hong (sha256 sai): $d"
        n=$((n+1))
    done < "$MANIFEST"
    log "goi cai dat: $n tep, sha256 dung"
}

# chen khoi lenh sau dong 'anchor' neu chua co
patch_hook() {  # patch_hook <file> <anchor-regex> <lenh>
    f="$1"; anchor="$2"; cmd="$3"
    [ -f "$f" ] || { log "bo qua $f (khong co)"; return 0; }
    if grep -q '\[tos' "$f"; then log "hook $f: da co TOS, giu nguyen"; return 0; fi
    c=$(grep -c "^$anchor\$" "$f")
    if [ "$c" != 1 ]; then log "hook $f: khong tim thay dung 1 diem chen ($c) -> bo qua (che do game tat)"; return 0; fi
    mkdir -p "$BK/hooks"; [ -f "$BK/hooks/$(basename "$f").orig" ] || cp -p "$f" "$BK/hooks/$(basename "$f").orig"
    printf '%s\n%s\n%s\n' "$BEGIN" "$cmd" "$END" > /tmp/tos_snip.$$
    sed -i "/^$anchor\$/r /tmp/tos_snip.$$" "$f" || die "khong va duoc $f"
    rm -f /tmp/tos_snip.$$
    grep -q 'tos-begin' "$f" || die "va $f that bai"
    log "hook $f: da chen"
    echo "$f" >> "$BK/patched.list"
}

do_install() {
    check_env
    msg "TOS $(ver): bat dau cai dat..." 2
    avail=$(df -k / | tail -1 | awk '{print $4}')
    [ "${avail:-0}" -ge 15000 ] || die "het cho trong (/ con ${avail} kB, can >= 15000)"
    verify_payload
    mkdir -p "$BK"; : > "$BK/created.list.new"; : > "$BK/replaced.list.new"
    [ -f "$BK/created.list" ] && cat "$BK/created.list" >> "$BK/created.list.new"
    [ -f "$BK/replaced.list" ] && cat "$BK/replaced.list" >> "$BK/replaced.list.new"
    msg "Dang chep tep he thong..." 1
    ncre=0; nrep=0; nskip=0
    while read -r t a b c d e; do
        case "$t" in
        D)  # D mode path
            if [ ! -d "$b" ] && [ ! -L "$b" ]; then mkdir -p "$b" && chmod "$a" "$b" && echo "D $b" >> "$BK/created.list.new"; fi ;;
        F)  # F mode sha256 size relpath dest
            dest="$e"
            if [ -f "$dest" ] && [ ! -L "$dest" ] && [ "$(sha256sum "$dest" | cut -d' ' -f1)" = "$b" ]; then nskip=$((nskip+1)); continue; fi
            if [ -e "$dest" ] || [ -L "$dest" ]; then
                if ! grep -q "^$dest\$" "$BK/replaced.list.new" && ! grep -q "^F $dest\$" "$BK/created.list.new"; then
                    mkdir -p "$BK/files$(dirname "$dest")"; cp -p "$dest" "$BK/files$dest" 2>/dev/null
                    echo "$dest" >> "$BK/replaced.list.new"
                fi
                nrep=$((nrep+1))
            else
                echo "F $dest" >> "$BK/created.list.new"; ncre=$((ncre+1))
            fi
            cp "$PAY/$d" "$dest.tos_new" && chmod "$a" "$dest.tos_new" && mv -f "$dest.tos_new" "$dest" || die "khong ghi duoc $dest" ;;
        L)  # L dest target
            if [ -L "$a" ] && [ "$(readlink "$a")" = "$b" ]; then nskip=$((nskip+1)); continue; fi
            if [ -e "$a" ] || [ -L "$a" ]; then log "bo qua symlink $a (da co)"; continue; fi
            ln -s "$b" "$a" && echo "L $a" >> "$BK/created.list.new" && ncre=$((ncre+1)) ;;
        esac
    done < "$MANIFEST"
    mv -f "$BK/created.list.new" "$BK/created.list"; mv -f "$BK/replaced.list.new" "$BK/replaced.list"
    log "tep: moi $ncre, ghi de $nrep (da luu ban goc), giu nguyen $nskip"

    # /run -> /var/run (libseat/Weston can /run)
    if [ ! -e /run ] && [ ! -L /run ]; then ln -s /var/run /run && echo "L /run" >> "$BK/created.list"; log "tao symlink /run"; fi

    # moc vao preload.sh / premainui.sh (chay sau khi MainUI thoat / truoc khi MainUI chay lai)
    patch_hook /usr/trimui/bin/preload.sh 'rm \/tmp\/stay_awake' '[ -f /tmp/cmd_to_run.sh ] && [ -x /usr/trimui/bin/tos_gamemode.sh ] && /usr/trimui/bin/tos_gamemode.sh on'
    patch_hook /usr/trimui/bin/premainui.sh 'rm -f \/tmp\/trimui_inputd\/input_dpad_to_joystick' '[ -x /usr/trimui/bin/tos_gamemode.sh ] && /usr/trimui/bin/tos_gamemode.sh off'

    # ap dung ngay
    sysctl -p /etc/sysctl.d/90-trimui-tuning.conf >/dev/null 2>&1 && log "sysctl: da ap dung"
    udevadm control --reload-rules >/dev/null 2>&1; udevadm trigger --subsystem-match=input --action=change >/dev/null 2>&1

    mkdir -p "$(dirname "$STATE")"
    printf 'version=%s\ndate=%s\n' "$(ver)" "$(date '+%F %T')" > "$STATE"
    sync
    msg "Cai dat xong. Hay KHOI DONG LAI may de ap dung day du." 7
}

do_uninstall() {
    [ -f "$STATE" ] || [ -f "$BK/created.list" ] || { msg "TOS chua duoc cai bang bo cai dat nay." 4; exit 0; }
    msg "Dang go cai dat TOS..." 2
    # go moc khoi hook
    for f in /usr/trimui/bin/preload.sh /usr/trimui/bin/premainui.sh; do
        if grep -q 'tos-begin' "$f" 2>/dev/null; then sed -i "/# \[tos-begin\]/,/# \[tos-end\]/d" "$f" && log "hook $f: da go"; fi
    done
    # xoa tep da tao (tep truoc, thu muc sau, theo thu tu nguoc)
    if [ -f "$BK/created.list" ]; then
        grep -E '^(F|L) ' "$BK/created.list" | while read -r t p; do rm -f "$p"; done
        grep -E '^D ' "$BK/created.list" | sed '1!G;h;$!d' | while read -r t p; do rmdir "$p" 2>/dev/null; done
    fi
    # tra lai tep goc da bi ghi de
    if [ -f "$BK/replaced.list" ]; then
        while read -r p; do [ -f "$BK/files$p" ] && cp -p "$BK/files$p" "$p" && log "tra lai $p"; done < "$BK/replaced.list"
    fi
    rm -f "$STATE"; rmdir /usr/lib/tos 2>/dev/null
    rm -rf "$BK"
    sync
    msg "Go cai dat xong. Hay khoi dong lai may." 6
}

do_status() {
    if [ -f "$STATE" ]; then echo "TOS da cai: $(tr '\n' ' ' < "$STATE")"; else echo "TOS chua cai bang bo cai dat"; fi
    [ -f /etc/tos_version ] && echo "firmware: $(head -1 /etc/tos_version)"
    echo "ARMHF: $([ -e /lib/ld-linux-armhf.so.3 ] && echo co || echo khong)  glremote: $([ -x /usr/bin/glremote_run ] && echo co || echo khong)  che do game: $(/usr/trimui/bin/tos_gamemode.sh status 2>/dev/null | head -1)"
}

case "$1" in
    install)   do_install ;;
    uninstall) do_uninstall ;;
    status)    do_status ;;
    *) echo "dung: $0 install|uninstall|status"; exit 2 ;;
esac
