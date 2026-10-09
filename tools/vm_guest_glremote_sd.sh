#!/bin/sh
# Chay TRONG may ao: kiem thu GOI SD glremote (System/bin/glremote_run + System/glremote) tren rootfs tos-3.0 (KHONG co glremote trong firmware).
# Goi duoc giai nen vao /mnt/SDCARD (tmpfs) y nhu giai nen vao the SD.
PASS=0; FAIL=0
ok()   { echo "PASS $*"; PASS=$((PASS+1)); }
bad()  { echo "FAIL $*"; FAIL=$((FAIL+1)); }
chk()  { d="$1"; shift; if "$@" >/dev/null 2>&1; then ok "$d"; else bad "$d"; fi; }

mkdir -p /tmp /mnt
mount -t proc p /proc 2>/dev/null; mount -t sysfs s /sys 2>/dev/null; mount -t devtmpfs d /dev 2>/dev/null
echo "nhan Linux: $(uname -r) ($(uname -m))"
mkdir -p /r
for m in crc16 crc32c_generic libcrc32c mbcache jbd2 ext4; do insmod /mods/$m.ko 2>&1 | head -1; done
if mount -t ext4 -o ro /dev/vdb /r 2>/tmp/m.err; then ok "mount rootfs (ext4, ro)"; else bad "mount rootfs: $(cat /tmp/m.err)"; echo "SUMMARY pass=$PASS fail=$FAIL"; echo VMTEST_DONE; exit 1; fi
echo "phien ban: $(tr '\n' ' ' < /r/etc/tos_version)"
mount -t tmpfs t /r/tmp; mount -t tmpfs t /r/mnt
mount --bind /proc /r/proc; mount --bind /sys /r/sys; mount --bind /dev /r/dev
mkdir -p /r/tmp/run /r/mnt/SDCARD
tar -xf /mods/sdpack.tar -C /r/mnt/SDCARD
S=/r/mnt/SDCARD/System/glremote

chk "rootfs KHONG co glremote trong firmware (dung de thu goi SD)" sh -c '[ ! -e /r/usr/lib/glremote/glserver ]'
chk "rootfs co ARMHF (ld-linux-armhf)"               [ -e /r/lib/ld-linux-armhf.so.3 ]
chk "goi SD: System/bin/glremote_run"                [ -f /r/mnt/SDCARD/System/bin/glremote_run ]
chk "goi SD: glserver, libglremote, libdrm, libasound, SDL2 du phong" sh -c "[ -f $S/glserver ] && [ -f $S/lib32/libglremote.so ] && [ -f $S/lib32/libdrm.so.2 ] && [ -f $S/lib32/libasound.so.2 ] && [ -f $S/lib32-sdl2/libSDL2-2.0.so.0 ]"
chk "goi SD: ban sao (khong symlink) libEGL.so.1, libGLESv2.so.2, libgbm.so.1" sh -c "[ -f $S/lib32/libEGL.so.1 ] && [ ! -L $S/lib32/libEGL.so.1 ] && [ -f $S/lib32/libGLESv2.so.2 ] && [ -f $S/lib32/libgbm.so.1 ]"
chk "goi SD: libEGL.so.1 giong libglremote.so (cmp)" cmp $S/lib32/libEGL.so.1 $S/lib32/libglremote.so
chk "sh -n glremote_run (busybox that)"              chroot /r /bin/sh -n /mnt/SDCARD/System/bin/glremote_run
chk "glremote_run dung System/glremote khi firmware khong co" grep -q 'System/glremote' /r/mnt/SDCARD/System/bin/glremote_run

cp /mods/selftest /r/tmp/selftest; chmod +x /r/tmp/selftest
chroot /r /bin/sh /mnt/SDCARD/System/bin/glremote_run /tmp/__khong_co__ > /tmp/miss0.out 2>&1
chroot /r /bin/sh -c 'cd /tmp; GLR_NULL=1 GLR_MAX=150 /bin/sh /mnt/SDCARD/System/bin/glremote_run /tmp/selftest' > /tmp/self.out 2>&1
RC=$?
grep -v 'no version' /tmp/self.out | head -40
echo "--- ma thoat: $RC"
echo "--- glserver.log:"; grep -v '^op ' /r/tmp/glserver.log | tail -6
echo "--- glaudio.log:"; tail -3 /r/tmp/glaudio.log 2>/dev/null
chk "chay tu goi SD: SDL chon KMSDRM_LEGACY"         grep -q 'SDL video driver: KMSDRM_LEGACY' /tmp/self.out
chk "chay tu goi SD: ve du 150 khung"                 grep -q 'SELFTEST frames=150' /tmp/self.out
chk "chay tu goi SD: callback am thanh duoc goi"      sh -c 'n=$(sed -n "s/.*audio_cb=\([0-9]*\).*/\1/p" /tmp/self.out | head -1); [ "${n:-0}" -ge 5 ]'
chk "chay tu goi SD: glserver nhan >= 500 lenh"       sh -c 'l=$(grep "^\[t+" /r/tmp/glserver.log | tail -1); c=$(echo "$l" | sed "s/.*lenh=\([0-9]*\) .*/\1/"); [ "${c:-0}" -ge 500 ]'
chk "chay tu goi SD: tien trinh am thanh nhan 48000 Hz 2 kenh" grep -q 'kenh=2 toc do=48000' /r/tmp/glaudio.log
chk "ma thoat 0, hoac 139 chi khi sap o QuitSubSystem(JOYSTICK) (loi da biet)" sh -c "[ \"$RC\" = 0 ] || { [ \"$RC\" = 139 ] && grep -q 'STEP quit joystick' /tmp/self.out; }"
sleep 1
chk "don dep: khong con glserver, /dev/dri/card0, bo nho chung" sh -c '! pgrep glserver && [ ! -e /dev/dri/card0 ] && [ ! -e /r/tmp/glremote.shm ] && [ ! -e /r/tmp/glremote.snd ]'

echo "SUMMARY pass=$PASS fail=$FAIL"
echo "VMTEST_DONE"
