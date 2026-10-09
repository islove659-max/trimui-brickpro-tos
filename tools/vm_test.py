"""Chạy rootfs v2 trong máy ảo QEMU (aarch64, giả lập) và kiểm thử bằng các nhị phân THẬT trong rootfs.
Nhân + initramfs: Alpine 'virt' (chỉ để khởi động một nhân Linux; hệ thống được thử là rootfs của Brick Pro, glibc 2.33).
KHÔNG giả lập được phần cứng A133 (GPU/màn hình/âm thanh/tay cầm) và không dùng bootloader/nhân của hãng.
Dùng: python -I -X utf8 vm_test.py <rootfs.fex>   (rootfs mở ở chế độ snapshot: KHÔNG ghi vào ảnh)
Kịch bản kiểm thử: tools/vm_guest_tests.sh (nạp qua đĩa ảo phụ /dev/vdb, nên không phải gõ vào cổng nối tiếp)."""
import os, queue, re, subprocess, sys, threading, time

QEMU = r'C:\Users\X\tools\qemu\qemu-system-aarch64.exe'
DL = r'C:\Users\X\tools\vm_dl'
HERE = os.path.dirname(os.path.abspath(__file__))
OUT = os.path.abspath(os.path.join(HERE, '..', 'out', os.environ.get('VM_OUT', 'fw_v2')))
LOG = os.path.join(OUT, 'vm_test.log')
IMG = r'C:\Users\X\tools\vm\test.img'

rootfs = sys.argv[1]
script = open(sys.argv[2] if len(sys.argv) > 2 else os.path.join(HERE, 'vm_guest_tests.sh'), 'rb').read().replace(b'\r\n', b'\n')
import io, tarfile
MODS = [r'crc16.ko', r'crc32c_generic.ko', r'libcrc32c.ko', r'mbcache.ko', r'jbd2.ko', r'ext4.ko', r'evdev.ko', r'sleeper']   # thứ tự phụ thuộc
bio = io.BytesIO()
with tarfile.open(fileobj=bio, mode='w', format=tarfile.USTAR_FORMAT) as tf:
    ti = tarfile.TarInfo('t.sh'); ti.size = len(script); ti.mode = 0o755; tf.addfile(ti, io.BytesIO(script))
    for m in MODS:
        data = open(os.path.join(DL, 'mods', m), 'rb').read()
        ti = tarfile.TarInfo('mods/' + m); ti.size = len(data); ti.mode = 0o644; tf.addfile(ti, io.BytesIO(data))
    # tep bo sung (vi du ung dung thu 32-bit) trong out/vm_extra -> /mods/<ten> tren dia ao
    extra = os.path.join(HERE, '..', 'out', 'vm_extra')
    if os.path.isdir(extra):
        for n in sorted(os.listdir(extra)):
            data = open(os.path.join(extra, n), 'rb').read()
            ti = tarfile.TarInfo('mods/' + n); ti.size = len(data); ti.mode = 0o755; tf.addfile(ti, io.BytesIO(data))
blob = bio.getvalue()
pad = (-len(blob)) % 512 or 512
open(IMG, 'wb').write(blob + b'\0' * pad)

cmd = [QEMU, '-M', 'virt', '-cpu', 'cortex-a53', '-smp', '2', '-m', '1024', '-nographic',
       '-kernel', os.path.join(DL, 'vmlinuz-virt'), '-initrd', os.path.join(DL, 'initramfs-virt'),
       '-append', 'console=ttyAMA0 rdinit=/bin/sh',
       '-drive', f'file={rootfs},if=none,id=hd0,format=raw,snapshot=on', '-device', 'virtio-blk-device,drive=hd0',
       '-drive', f'file={IMG},if=none,id=hd1,format=raw,readonly=on', '-device', 'virtio-blk-device,drive=hd1',
       '-device', 'virtio-keyboard-device', '-serial', 'stdio', '-monitor', 'none', '-net', 'none']
print('khởi động QEMU (giả lập aarch64, có thể mất vài phút)...')
t_start = time.time()
p = subprocess.Popen(cmd, stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
q = queue.Queue()
threading.Thread(target=lambda: [q.put(c) for c in iter(lambda: p.stdout.read(1), b'')], daemon=True).start()
buf = bytearray()

def pump(until, timeout):
    t0 = time.time()
    while time.time() - t0 < timeout:
        try: buf.extend(q.get(timeout=0.3))
        except queue.Empty: pass
        if re.search(until, buf.decode('utf8', 'replace')): return True
    return False

def typ(s):
    for ch in s + '\n':
        p.stdin.write(ch.encode()); p.stdin.flush(); time.sleep(0.012)

if not pump(r'# $', 180):
    print('KHÔNG thấy dấu nhắc shell. Đầu ra:'); print(buf.decode('utf8', 'replace')[-1500:]); p.kill(); sys.exit(2)
print('nhân đã khởi động sau %.0fs' % (time.time() - t_start))
time.sleep(1)
typ('/bin/busybox --install -s /bin; mount -t proc p /proc; mount -t devtmpfs d /dev; modprobe virtio_mmio; modprobe virtio_blk; sleep 3; ls /dev/vd*')
time.sleep(3); buf.clear()
typ('mkdir -p /tmp; tar -xf /dev/vda -C /; sh /t.sh')
ok = pump('VMTEST_DONE', 420)
text = buf.decode('utf8', 'replace').replace('\r', '')
lines = [l for l in text.split('\n') if re.match(r'^(PASS|FAIL|---|nhan Linux|he thong|phien ban|GM\d|SUMMARY)', l)]
report = '\n'.join(lines)
print(report)
open(LOG, 'w', encoding='utf-8').write(report + '\n\n==== RAW ====\n' + text)
p.kill()
print('\nthời gian: %.0fs | hoàn tất: %s | log: %s' % (time.time() - t_start, ok, LOG))
sys.exit(0 if ok and 'fail=0' in report else 1)
