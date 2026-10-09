#!/bin/sh
# Chay TRONG may ao: kiem thu BO CAI DAT TOS tren rootfs Stock goc (hoac rootfs da co TOS). Rootfs mo ghi (anh chay che do snapshot, KHONG ghi that).
PASS=0; FAIL=0
ok()   { echo "PASS $*"; PASS=$((PASS+1)); }
bad()  { echo "FAIL $*"; FAIL=$((FAIL+1)); }
chk()  { d="$1"; shift; if "$@" >/dev/null 2>&1; then ok "$d"; else bad "$d"; fi; }

mkdir -p /tmp /mnt
mount -t proc p /proc 2>/dev/null; mount -t sysfs s /sys 2>/dev/null; mount -t devtmpfs d /dev 2>/dev/null
echo "nhan Linux: $(uname -r) ($(uname -m))"
mkdir -p /r
for m in crc16 crc32c_generic libcrc32c mbcache jbd2 ext4; do insmod /mods/$m.ko 2>&1 | head -1; done
if mount -t ext4 -o rw /dev/vdb /r 2>/tmp/m.err; then ok "mount rootfs (ext4, rw trong snapshot)"; else bad "mount rootfs: $(cat /tmp/m.err)"; echo "SUMMARY pass=$PASS fail=$FAIL"; echo VMTEST_DONE; exit 1; fi
HASTOS=0; [ -f /r/etc/tos_version ] && HASTOS=1; export HASTOS
echo "rootfs: $( [ $HASTOS = 1 ] && tr '\n' ' ' < /r/etc/tos_version || echo 'Stock goc (khong co TOS)')"
mount -t tmpfs t /r/tmp; mount -t tmpfs t /r/mnt
mount --bind /proc /r/proc; mount --bind /sys /r/sys; mount --bind /dev /r/dev
mkdir -p /r/tmp/run /r/mnt/SDCARD /r/mnt/UDISK
tar -xf /mods/sdroot.tar -C /r/mnt/SDCARD
cp /mods/selftest /r/tmp/selftest; chmod +x /r/tmp/selftest
I="sh /mnt/SDCARD/System/tos/install.sh"
export TOS_SCREEN=0

cp /r/usr/trimui/bin/preload.sh /tmp/pre.orig; cp /r/usr/trimui/bin/premainui.sh /tmp/pm.orig
chk "goi: install.sh, payload/MANIFEST, 2 app co mat"   sh -c '[ -f /r/mnt/SDCARD/System/tos/install.sh ] && [ -f /r/mnt/SDCARD/System/tos/payload/MANIFEST ] && [ -f /r/mnt/SDCARD/Apps/TOS_CaiDat/launch.sh ] && [ -f /r/mnt/SDCARD/Apps/TOS_GoCaiDat/config.json ]'
chk "sh -n install.sh + launch.sh (busybox that)"       sh -c 'chroot /r /bin/sh -n /mnt/SDCARD/System/tos/install.sh && chroot /r /bin/sh -n /mnt/SDCARD/Apps/TOS_CaiDat/launch.sh'
if [ $HASTOS = 0 ]; then
  chk "TRUOC: chua co ld-linux-armhf, chua co glremote"   sh -c '[ ! -e /r/lib/ld-linux-armhf.so.3 ] && [ ! -e /r/usr/bin/glremote_run ]'
  chk "TRUOC: hook chua co [tos]"                          sh -c '! grep -q "\[tos" /r/usr/trimui/bin/preload.sh'
fi

# ---------- cai dat ----------
chroot /r $I status > /tmp/st0.out 2>&1; echo "--- status truoc: $(tr '\n' ';' < /tmp/st0.out | cut -c1-200)"
chroot /r $I install > /tmp/i1.out 2>&1; RC=$?
echo "--- install lan 1: ma=$RC"; grep -v '^$' /tmp/i1.out | tail -12
chk "install lan 1 thanh cong (ma 0)"                    [ "$RC" = 0 ]
chk "ARMHF: ld-linux-armhf.so.3 co + chay libc 32-bit"   sh -c 'chroot /r /lib/arm-linux-gnueabihf/libc.so.6 2>&1 | grep -q "GNU C Library"'
chk "glremote: glremote_run + glserver co"               sh -c '[ -x /r/usr/bin/glremote_run ] && [ -x /r/usr/lib/glremote/glserver ]'
chk "udev rule + sysctl + tos_gamemode + tos_env co"     sh -c '[ -f /r/etc/udev/rules.d/61-trimui-input.rules ] && [ -f /r/etc/sysctl.d/90-trimui-tuning.conf ] && [ -x /r/usr/trimui/bin/tos_gamemode.sh ] && [ -f /r/usr/lib/tos/tos_env.sh ]'
chk "preload.sh / premainui.sh con cu phap dung (sh -n)" sh -c 'chroot /r /bin/sh -n /usr/trimui/bin/preload.sh && chroot /r /bin/sh -n /usr/trimui/bin/premainui.sh'
chk "hook co goi tos_gamemode (on / off)"                sh -c 'grep -q "tos_gamemode.sh on" /r/usr/trimui/bin/preload.sh && grep -q "tos_gamemode.sh off" /r/usr/trimui/bin/premainui.sh'
chk "hook giu lenh goc (echo ondemand)"                  grep -q 'echo ondemand' /r/usr/trimui/bin/preload.sh
chk "/run -> /var/run"                                   sh -c '[ -L /r/run ] || [ -d /r/run ]'
chk "ban sao luu hook goc o /mnt/UDISK/tos/backup"       sh -c '[ $HASTOS = 1 ] || { [ -f /r/mnt/UDISK/tos/backup/hooks/preload.sh.orig ] && cmp /r/mnt/UDISK/tos/backup/hooks/preload.sh.orig /tmp/pre.orig; }'
chroot /r $I status > /tmp/st1.out 2>&1; echo "--- status sau: $(tr '\n' ';' < /tmp/st1.out | cut -c1-240)"
chk "status bao da cai"                                  grep -q 'TOS da cai' /tmp/st1.out

# ---------- chay that: game 32-bit qua glremote (may chu --null) ----------
chroot /r /bin/sh -c 'cd /tmp; GLR_NULL=1 GLR_MAX=150 /usr/bin/glremote_run /tmp/selftest' > /tmp/self.out 2>&1; RC2=$?
grep -E 'SELFTEST|SDL video' /tmp/self.out | head -4
chk "sau cai: game 32-bit ve du 150 khung + am thanh"    sh -c 'grep -q "SELFTEST frames=150" /tmp/self.out && n=$(sed -n "s/.*audio_cb=\([0-9]*\).*/\1/p" /tmp/self.out | head -1) && [ "${n:-0}" -ge 5 ]'

# ---------- che do game voi tien trinh that ----------
cp /mods/sleeper /r/tmp/sftpgo; cp /mods/sleeper /r/tmp/MtpDaemon; chmod +x /r/tmp/sftpgo /r/tmp/MtpDaemon
chroot /r /bin/sh -c '
cd /tmp; /tmp/sftpgo & /tmp/MtpDaemon &
sleep 1
touch /tmp/cmd_to_run.sh
sh /usr/trimui/bin/preload.sh >/dev/null 2>&1; sleep 1
echo "GM1 sftpgo=[$(pgrep sftpgo)] mtp=[$(pgrep MtpDaemon)] status=$(sh /usr/trimui/bin/tos_gamemode.sh status | cut -c1-3)"
sh /usr/trimui/bin/premainui.sh >/dev/null 2>&1; sleep 2
echo "GM2 sftpgo=[$(pgrep sftpgo)] mtp=[$(pgrep MtpDaemon)] status=$(sh /usr/trimui/bin/tos_gamemode.sh status | cut -c1-3)"
rm -f /tmp/cmd_to_run.sh' > /tmp/gm.out 2>&1
cat /tmp/gm.out | grep '^GM'
chk "che do game (qua hook da cai): vao game dung 2 dich vu"  grep -q '^GM1 sftpgo=\[\] mtp=\[\] status=ON' /tmp/gm.out
chk "che do game: ve menu bat lai 2 dich vu"                  sh -c 'grep "^GM2" /tmp/gm.out | grep -qE "sftpgo=\[[0-9]+\] mtp=\[[0-9]+\] status=OFF"'
killall sftpgo MtpDaemon 2>/dev/null

# ---------- chay lai (cap nhat): khong doi gi ----------
cp /r/usr/trimui/bin/preload.sh /tmp/pre.after1; cp /r/usr/trimui/bin/premainui.sh /tmp/pm.after1
chroot /r $I install > /tmp/i2.out 2>&1; RC3=$?
echo "--- install lan 2: ma=$RC3: $(grep 'tep:' /tmp/i2.out)"
chk "install lan 2 (cap nhat) ma 0"                      [ "$RC3" = 0 ]
chk "install lan 2: khong chen hook lan nua"             sh -c 'cmp /r/usr/trimui/bin/preload.sh /tmp/pre.after1 && cmp /r/usr/trimui/bin/premainui.sh /tmp/pm.after1'
chk "install lan 2: moi 0, ghi de 0"                     grep -q 'moi 0, ghi de 0' /tmp/i2.out

# ---------- go cai dat ----------
chroot /r $I uninstall > /tmp/u.out 2>&1; RCU=$?
echo "--- uninstall: ma=$RCU"; grep -v '^$' /tmp/u.out | tail -6
chk "uninstall ma 0"                                      [ "$RCU" = 0 ]
chk "uninstall: hook tro ve ban goc (cmp)"                sh -c 'cmp /r/usr/trimui/bin/preload.sh /tmp/pre.orig && cmp /r/usr/trimui/bin/premainui.sh /tmp/pm.orig'
if [ $HASTOS = 0 ]; then
  chk "uninstall: da go glremote, ARMHF, udev, sysctl, gamemode" sh -c '[ ! -e /r/usr/lib/glremote ] && [ ! -e /r/usr/bin/glremote_run ] && [ ! -e /r/lib/ld-linux-armhf.so.3 ] && [ ! -e /r/lib/arm-linux-gnueabihf ] && [ ! -e /r/etc/udev/rules.d/61-trimui-input.rules ] && [ ! -e /r/etc/sysctl.d/90-trimui-tuning.conf ] && [ ! -e /r/usr/trimui/bin/tos_gamemode.sh ]'
  chk "uninstall: /run symlink do TOS da go"            sh -c '[ ! -e /r/run ] && [ ! -L /r/run ]'
else
  chk "uninstall tren firmware da co TOS: khong go tep cua firmware" sh -c '[ -x /r/usr/bin/glremote_run ] && [ -e /r/lib/ld-linux-armhf.so.3 ] && grep -q "\[tos" /r/usr/trimui/bin/preload.sh'
fi
chroot /r $I status > /tmp/st2.out 2>&1
chk "status sau go: chua cai"                             grep -q 'chua cai' /tmp/st2.out
# cai lai sau khi go
chroot /r $I install > /tmp/i3.out 2>&1
chk "cai lai sau khi go: thanh cong"                      sh -c 'chroot /r /bin/sh -c "[ -x /usr/bin/glremote_run ]" && grep -q "tos_gamemode.sh on" /r/usr/trimui/bin/preload.sh'

echo "SUMMARY pass=$PASS fail=$FAIL"
echo "VMTEST_DONE"
