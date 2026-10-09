#!/bin/sh
# Chạy TRONG máy ảo (busybox của initramfs Alpine). Gắn rootfs v2 (ro) rồi chroot chạy nhị phân thật của Brick Pro.
# Mỗi kiểm tra in một dòng "PASS ..." hoặc "FAIL ..."; dòng cuối "SUMMARY pass=N fail=M".
PASS=0; FAIL=0
ok()   { echo "PASS $*"; PASS=$((PASS+1)); }
bad()  { echo "FAIL $*"; FAIL=$((FAIL+1)); }
chk()  { d="$1"; shift; if "$@" >/dev/null 2>&1; then ok "$d"; else bad "$d"; fi; }

mkdir -p /tmp /mnt
mount -t proc p /proc 2>/dev/null; mount -t sysfs s /sys 2>/dev/null; mount -t devtmpfs d /dev 2>/dev/null
echo "nhan Linux: $(uname -r) ($(uname -m))"
mkdir -p /r
for m in crc16 crc32c_generic libcrc32c mbcache jbd2 ext4; do insmod /mods/$m.ko 2>&1 | head -1; done
if mount -t ext4 -o ro /dev/vdb /r 2>/tmp/m.err; then ok "mount rootfs v2 (ext4, ro) bang nhan Linux that"; else bad "mount rootfs: $(cat /tmp/m.err)"; echo "SUMMARY pass=$PASS fail=$FAIL"; echo VMTEST_DONE; exit 1; fi
echo "he thong: $(sed -n 1,2p /r/etc/openwrt_release | tr '\n' ' ')"
echo "phien ban: $(tr '\n' ' ' < /r/etc/tos_version)"

mount -t tmpfs t /r/tmp; mount -t tmpfs t /r/mnt
mount --bind /proc /r/proc; mount --bind /sys /r/sys; mount --bind /dev /r/dev
mkdir -p /r/tmp/run /r/mnt/UDISK/tos

# ---------- 1. tep va quyen ----------
chk "/run la symlink -> /var/run"           chroot /r /bin/sh -c '[ -L /run ] && [ "$(readlink /run)" = /var/run ]'
chk "/run phan giai duoc (qua /var/run)"    chroot /r /bin/sh -c 'touch /run/_t && rm /run/_t'
chk "luat udev 61-trimui-input.rules co"    [ -f /r/etc/udev/rules.d/61-trimui-input.rules ]
chk "sysctl.d/90-trimui-tuning.conf co"     [ -f /r/etc/sysctl.d/90-trimui-tuning.conf ]
chk "tos_gamemode.sh co + thuc thi"         [ -x /r/usr/trimui/bin/tos_gamemode.sh ]
chk "tos_env.sh co"                         [ -f /r/usr/lib/tos/tos_env.sh ]
chk "preload.sh/premainui.sh con quyen thuc thi" sh -c '[ -x /r/usr/trimui/bin/preload.sh ] && [ -x /r/usr/trimui/bin/premainui.sh ]'
chk "preload.sh co moc [tos]"               grep -q '\[tos\]' /r/usr/trimui/bin/preload.sh
chk "premainui.sh co moc [tos]"             grep -q '\[tos\]' /r/usr/trimui/bin/premainui.sh
chk "preload.sh giu lenh goc (governor ondemand)" grep -q 'echo ondemand' /r/usr/trimui/bin/preload.sh
chk "fstab the SD VAN CON sync (khong doi)" sh -c 'grep -c "rw,sync" /r/etc/config/fstab | grep -qv "^0$"'
chk "KHONG co swap trong thay doi moi"      sh -c '! grep -qs "swapon\|swapfile" /r/usr/trimui/bin/tos_gamemode.sh /r/etc/sysctl.d/90-trimui-tuning.conf /r/etc/udev/rules.d/61-trimui-input.rules'

# ---------- 2. cu phap script bang busybox/ash that cua may ----------
for f in /usr/trimui/bin/preload.sh /usr/trimui/bin/premainui.sh /usr/trimui/bin/tos_gamemode.sh /usr/lib/tos/tos_env.sh /usr/trimui/bin/runtrimui.sh; do
  chk "sh -n $f" chroot /r /bin/sh -n $f
done

# ---------- 3. sysctl that ----------
chroot /r /bin/sh -c 'sysctl -p /etc/sysctl.d/90-trimui-tuning.conf' > /tmp/sysctl.out 2>&1
echo "--- sysctl: $(tr '\n' ';' < /tmp/sysctl.out | cut -c1-300)"
chk "sysctl: fs.inotify.max_user_watches=65536" sh -c '[ "$(cat /proc/sys/fs/inotify/max_user_watches)" = 65536 ]'
chk "sysctl: vm.vfs_cache_pressure=50"      sh -c '[ "$(cat /proc/sys/vm/vfs_cache_pressure)" = 50 ]'
chk "sysctl: kernel.printk = 4 4 1 7"       sh -c '[ "$(cat /proc/sys/kernel/printk | tr "\t" " ")" = "4 4 1 7" ]'
chk "sysctl: net.core.rmem_max=1048576"     sh -c '[ "$(cat /proc/sys/net/core/rmem_max)" = 1048576 ]'

# ---------- 4. udev that: phan tich luat ----------
chroot /r /sbin/udevd -d >/tmp/udevd.out 2>&1; sleep 2
chk "udevd (nhi phan that cua may) chay duoc" sh -c 'grep -l udevd /proc/[0-9]*/comm 2>/dev/null | head -1 | grep -q .'
chroot /r /bin/sh -c 'udevadm test /sys/class/block/vda' > /tmp/udevtest.out 2>&1
echo "--- udevadm test: dong nhac toi luat TOS: $(grep -c '61-trimui' /tmp/udevtest.out); dong loi lien quan luat TOS: $(grep -i '61-trimui' /tmp/udevtest.out | grep -ciE 'error|invalid|unknown|syntax')"
chk "udev: luat TOS duoc doc" grep -q '61-trimui-input.rules' /tmp/udevtest.out
chk "udev: luat TOS khong co loi cu phap" sh -c '! grep -i "61-trimui" /tmp/udevtest.out | grep -iE "error|invalid|unknown|syntax" | grep -q .'
echo "--- (khong thu khop thiet bi input: nhan Alpine khong gan driver virtio_input; khop luat da kiem tren may that bang libinput)"
chroot /r /bin/sh -c 'udevadm control --exit' >/dev/null 2>&1

# ---------- 5. che do game voi NHI PHAN THAT ten sftpgo/MtpDaemon ----------
cp /mods/sleeper /r/tmp/sftpgo; cp /mods/sleeper /r/tmp/MtpDaemon; chmod +x /r/tmp/sftpgo /r/tmp/MtpDaemon
chroot /r /bin/sh -c '
cd /tmp
/tmp/sftpgo & /tmp/MtpDaemon &
sleep 1
echo "GM0 sftpgo=[$(pgrep sftpgo)] mtp=[$(pgrep MtpDaemon)]"
touch /tmp/cmd_to_run.sh
sh /usr/trimui/bin/preload.sh >/dev/null 2>&1
sleep 1
echo "GM1 sftpgo=[$(pgrep sftpgo)] mtp=[$(pgrep MtpDaemon)] status=$(sh /usr/trimui/bin/tos_gamemode.sh status | cut -c1-3)"
sh /usr/trimui/bin/premainui.sh >/dev/null 2>&1
sleep 2
echo "GM2 sftpgo=[$(pgrep sftpgo)] mtp=[$(pgrep MtpDaemon)] status=$(sh /usr/trimui/bin/tos_gamemode.sh status | cut -c1-3)"
rm -f /tmp/cmd_to_run.sh
sh /usr/trimui/bin/preload.sh >/dev/null 2>&1
echo "GM3 (khong co cmd_to_run) sftpgo=[$(pgrep sftpgo)] status=$(sh /usr/trimui/bin/tos_gamemode.sh status | cut -c1-3)"
echo GAMEMODE=0 > /mnt/UDISK/tos/tos.conf
touch /tmp/cmd_to_run.sh
sh /usr/trimui/bin/preload.sh >/dev/null 2>&1
echo "GM4 (GAMEMODE=0) sftpgo=[$(pgrep sftpgo)] status=$(sh /usr/trimui/bin/tos_gamemode.sh status | cut -c1-3)"
' > /tmp/gm.out 2>&1
cat /tmp/gm.out
chk "che do game: truoc khi vao game 2 dich vu dang chay" sh -c 'grep "^GM0" /tmp/gm.out | grep -qE "sftpgo=\[[0-9]+\] mtp=\[[0-9]+\]"'
chk "che do game: vao game (preload) => dung ca 2, trang thai ON" sh -c 'grep "^GM1" /tmp/gm.out | grep -q "sftpgo=\[\] mtp=\[\] status=ON"'
chk "che do game: ve menu (premainui) => bat lai ca 2, trang thai OFF" sh -c 'grep "^GM2" /tmp/gm.out | grep -qE "sftpgo=\[[0-9]+\] mtp=\[[0-9]+\] status=OFF"'
chk "che do game: KHONG co cmd_to_run => khong dung gi"   sh -c 'grep "^GM3" /tmp/gm.out | grep -qE "sftpgo=\[[0-9]+\] status=OFF"'
chk "che do game: GAMEMODE=0 => tat han"                  sh -c 'grep "^GM4" /tmp/gm.out | grep -qE "sftpgo=\[[0-9]+\] status=OFF"'
echo "--- busybox cua rootfs: $(chroot /r /bin/sh -c 'busybox 2>&1 | head -1')"
chk "tos_gamemode.sh khong dung find -delete (busybox 1.27.2 khong ho tro)" sh -c '! grep -v "^ *#" /r/usr/trimui/bin/tos_gamemode.sh | grep -q -- "-delete"'

# ---------- 5b. ARMHF 32-bit (chi co o ban tos-3.0) ----------
if [ -f /r/lib/ld-linux-armhf.so.3 ] || [ -L /r/lib/ld-linux-armhf.so.3 ]; then
  echo "--- ARMHF: co /lib/ld-linux-armhf.so.3 -> $(readlink /r/lib/ld-linux-armhf.so.3)"
  chk "armhf: /lib/ld-linux-armhf.so.3 -> arm-linux-gnueabihf/ld-2.31.so" sh -c '[ "$(readlink /r/lib/ld-linux-armhf.so.3)" = "arm-linux-gnueabihf/ld-2.31.so" ]'
  chk "armhf: ld-2.31.so la ELF 32-bit ARM (e_machine=40)" sh -c '[ "$(od -An -tu1 -j4 -N1 /r/lib/arm-linux-gnueabihf/ld-2.31.so | tr -d " ")" = 1 ] && [ "$(od -An -tu2 -j18 -N2 /r/lib/arm-linux-gnueabihf/ld-2.31.so | tr -d " ")" = 40 ]'
  chk "armhf: du libc/libm/libpthread/libdl/librt/libgcc_s/libstdc++" sh -c 'for f in /r/lib/arm-linux-gnueabihf/libc.so.6 /r/lib/arm-linux-gnueabihf/libm.so.6 /r/lib/arm-linux-gnueabihf/libpthread.so.0 /r/lib/arm-linux-gnueabihf/libdl.so.2 /r/lib/arm-linux-gnueabihf/librt.so.1 /r/lib/arm-linux-gnueabihf/libgcc_s.so.1 /r/usr/lib/arm-linux-gnueabihf/libstdc++.so.6; do [ -e "$f" ] || exit 1; done'
  chroot /r /lib/arm-linux-gnueabihf/libc.so.6 > /tmp/armhf_libc.out 2>&1
  echo "--- chay libc.so.6 32-bit qua loader armhf: $(head -c 160 /tmp/armhf_libc.out | head -1)"
  chk "armhf: CHAY duoc chuong trinh 32-bit qua /lib/ld-linux-armhf.so.3 (libc banner)" grep -q 'GNU C Library' /tmp/armhf_libc.out
  chroot /r /lib/ld-linux-armhf.so.3 --list /lib/arm-linux-gnueabihf/libc.so.6 > /tmp/armhf_ldlist.out 2>&1
  echo "--- ld --list: $(tr '\n' ';' < /tmp/armhf_ldlist.out | cut -c1-200)"
  chk "armhf: loader 32-bit tu chay duoc (--list)" sh -c 'grep -q "libc.so.6\|ld-linux\|ld-2.31" /tmp/armhf_ldlist.out'
  chk "64-bit KHONG bi anh huong (busybox aarch64 van chay)" chroot /r /bin/sh -c 'echo ok'
else
  echo "--- (bo qua) rootfs nay khong co ARMHF"
fi

# ---------- 6. tos_env.sh ----------
chroot /r /bin/sh -c '. /usr/lib/tos/tos_env.sh; tos_detect; echo "TOSENV os=$TOS_OS device=$TOS_DEVICE"' > /tmp/env.out 2>&1
echo "--- $(cat /tmp/env.out | tr '\n' ' ')"
chk "tos_env.sh nap + tos_detect chay duoc" grep -q '^TOSENV os=' /tmp/env.out

echo "SUMMARY pass=$PASS fail=$FAIL"
echo "VMTEST_DONE"
