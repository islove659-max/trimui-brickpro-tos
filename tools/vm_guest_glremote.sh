#!/bin/sh
# Chay TRONG may ao: kiem thu runtime glremote (tos-4.0) bang nhi phan THAT trong rootfs + ung dung thu 32-bit.
# GLR_NULL=1: glserver khong dung SDL/GPU/loa that (may ao khong co phan cung); toan bo duong shim <-> bo nho chung <-> may chu duoc chay that.
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
mkdir -p /r/tmp/run /r/mnt/UDISK/tos

G=/r/usr/lib/glremote
# ---------- 1. tep, quyen, symlink ----------
chk "tos_version co glremote: 1"                      grep -q '^glremote: 1' /r/etc/tos_version
chk "glserver co + thuc thi"                          [ -x $G/glserver ]
chk "glremote_run co + thuc thi"                      [ -x /r/usr/bin/glremote_run ]
chk "sh -n glremote_run (busybox that)"               chroot /r /bin/sh -n /usr/bin/glremote_run
chk "glserver la ELF aarch64 (e_machine=183)"         sh -c "[ \"\$(od -An -tu2 -j18 -N2 $G/glserver | tr -d \" \")\" = 183 ]"
chk "libglremote.so la ELF 32-bit ARM (e_machine=40)" sh -c "[ \"\$(od -An -tu1 -j4 -N1 $G/lib32/libglremote.so | tr -d \" \")\" = 1 ] && [ \"\$(od -An -tu2 -j18 -N2 $G/lib32/libglremote.so | tr -d \" \")\" = 40 ]"
for n in libEGL.so.1 libGLESv2.so.2 libGL.so.1 libGLESv1_CM.so.1; do
  chk "symlink $n -> libglremote.so" sh -c "[ \"\$(readlink $G/lib32/$n)\" = libglremote.so ]"
done
chk "symlink libgbm.so.1 -> libdrm.so.2"              sh -c "[ \"\$(readlink $G/lib32/libgbm.so.1)\" = libdrm.so.2 ]"
chk "symlink libX11.so.6 -> libsdl_deps_stub.so"      sh -c "[ \"\$(readlink $G/lib32/libX11.so.6)\" = libsdl_deps_stub.so ]"
chk "libasound.so.2 (ban gia) co"                     [ -f $G/lib32/libasound.so.2 ]
chk "libSDL2 32-bit du phong co"                      [ -f $G/lib32-sdl2/libSDL2-2.0.so.0 ]
chk "ARMHF nen tang (ld-linux-armhf) van con"         [ -e /r/lib/ld-linux-armhf.so.3 ]
chk "tep goc 64-bit khong bi doi: SDL2 he thong"      sh -c "ls /r/usr/trimui/lib/libSDL2-2.0.so.0 >/dev/null 2>&1 || ls /r/usr/lib/libSDL2-2.0.so.0 >/dev/null 2>&1"

# ---------- 2. ung dung thu 32-bit: nap thu vien ----------
cp /mods/selftest /r/tmp/selftest; chmod +x /r/tmp/selftest
LP="/usr/lib/glremote/lib32:/usr/lib/glremote/lib32-sdl2"
chroot /r /lib/ld-linux-armhf.so.3 --library-path "$LP" --list /tmp/selftest > /tmp/list.out 2>&1
echo "--- ld --list selftest: $(grep -c '=>' /tmp/list.out) thu vien; khong tim thay: $(grep -c 'not found' /tmp/list.out)"
chk "selftest 32-bit: moi phu thuoc tim thay (khong 'not found')" sh -c '! grep -q "not found" /tmp/list.out'
chk "selftest 32-bit: libSDL2 nap tu lib32-sdl2"      grep -q 'lib32-sdl2/libSDL2-2.0.so.0' /tmp/list.out
chk "selftest 32-bit: libasound la ban gia"           grep -q 'glremote/lib32/libasound.so.2' /tmp/list.out

# ---------- 3. chay that (may chu --null): GL + SDL KMSDRM gia + am thanh ----------
chroot /r /bin/sh -c 'cd /tmp; GLR_NULL=1 GLR_MAX=150 /usr/bin/glremote_run /tmp/selftest' > /tmp/self.out 2>&1
RC=$?
grep -v 'no version' /tmp/self.out | head -90
echo "--- ma thoat glremote_run/selftest: $RC"
echo "--- glserver.log:"; sed -n '1,200p' /r/tmp/glserver.log | grep -v '^op ' | tail -8
echo "--- glaudio.log:"; cat /r/tmp/glaudio.log 2>/dev/null | tail -4
chk "selftest tat audio+video+timer sach (STEP quit timer in ra)" grep -q "STEP quit timer" /tmp/self.out
chk "selftest ma thoat 0, hoac 139 CHI khi sap o QuitSubSystem(JOYSTICK) cua SDL2 Debian (loi da biet)" sh -c "[ \"$RC\" = 0 ] || { [ \"$RC\" = 139 ] && grep -q \"STEP quit joystick\" /tmp/self.out && ! grep -q \"STEP quit\$\" /tmp/self.out; }"
chk "selftest: SDL chon driver KMSDRM_LEGACY"         grep -q 'SDL video driver: KMSDRM_LEGACY' /tmp/self.out
chk "selftest: ve du 150 khung"                       grep -q 'SELFTEST frames=150' /tmp/self.out
chk "selftest: callback am thanh SDL (alsa gia) duoc goi" sh -c 'n=$(sed -n "s/.*audio_cb=\([0-9]*\).*/\1/p" /tmp/self.out | head -1); [ "${n:-0}" -ge 5 ]'
chk "selftest: mo duoc thiet bi am thanh"             sh -c 'n=$(sed -n "s/.*audio_dev=\([0-9]*\).*/\1/p" /tmp/self.out | head -1); [ "${n:-0}" -ge 2 ]'
chk "glserver nhan lenh GL (bao cao cuoi >= 60 khung, >= 500 lenh)" sh -c 'l=$(grep "^\[t+" /r/tmp/glserver.log | tail -1); k=$(echo "$l" | sed "s/.*khung=\([0-9]*\).*/\1/"); c=$(echo "$l" | sed "s/.*lenh=\([0-9]*\) .*/\1/"); echo "bao cao cuoi: $l" >&2; [ "${k:-0}" -ge 60 ] && [ "${c:-0}" -ge 500 ]'
chk "glserver KHONG bao lenh la / loi shader"        sh -c '! grep -qE "lenh la|SHADER LOI|LINK LOI" /r/tmp/glserver.log'
chk "tien trinh am thanh nhan cau hinh 48000 Hz 2 kenh" grep -q 'kenh=2 toc do=48000' /r/tmp/glaudio.log

# ---------- 4. don dep ----------
sleep 1
chk "khong con glserver chay"                         sh -c '! pgrep glserver'
chk "da xoa /dev/dri/card0 tao tam"                   sh -c '[ ! -e /dev/dri/card0 ]'
chk "da xoa bo nho chung /tmp/glremote.shm,.snd"      sh -c '[ ! -e /r/tmp/glremote.shm ] && [ ! -e /r/tmp/glremote.snd ]'

# ---------- 5. gioi han thoi gian (watchdog) + loi thieu ARMHF ----------
cp /mods/sleeper /r/tmp/sleeper32 2>/dev/null
t0=$(date +%s)
chroot /r /bin/sh -c 'GLR_NULL=1 GLR_MAX=4 /usr/bin/glremote_run /tmp/selftest_missing' > /tmp/miss.out 2>&1; RC2=$?
t1=$(date +%s)
echo "--- chay file khong ton tai: ma=$RC2 thoi gian=$((t1-t0))s: $(grep -v 'no version' /tmp/miss.out | head -2 | tr '\n' ' ')"
chk "app loi => glremote_run thoat gon (<=20s) va don dep" sh -c "[ $((t1-t0)) -le 20 ] && ! pgrep glserver"

echo "SUMMARY pass=$PASS fail=$FAIL"
echo "VMTEST_DONE"
