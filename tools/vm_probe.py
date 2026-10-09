"""Thăm dò máy ảo: initramfs Alpine có những module/thiết bị gì (gõ chậm để không rớt ký tự)."""
import queue, re, subprocess, sys, threading, time
QEMU = r'C:\Users\X\tools\qemu\qemu-system-aarch64.exe'; DL = r'C:\Users\X\tools\vm_dl'
cmd = [QEMU, '-M', 'virt', '-cpu', 'cortex-a53', '-smp', '2', '-m', '1024', '-nographic',
       '-kernel', DL + r'\vmlinuz-virt', '-initrd', DL + r'\initramfs-virt',
       '-append', 'console=ttyAMA0 rdinit=/bin/sh',
       '-drive', r'file=C:\Users\X\tools\vm\rootfs_v2.fex,if=none,id=hd0,format=raw,snapshot=on', '-device', 'virtio-blk-device,drive=hd0',
       '-device', 'virtio-keyboard-device', '-serial', 'stdio', '-monitor', 'none', '-net', 'none']
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
pump(r'# $', 90)
time.sleep(1); buf.clear()
typ('/bin/busybox --install -s /bin; mkdir -p /tmp; mount -t proc p /proc; mount -t devtmpfs d /dev; echo FS: $(cat /proc/filesystems | tr "\n" " "); echo END_A')
pump('END_A\r?\n', 30); print(re.sub(r'\n\s*\n', '\n', buf.decode('utf8','replace'))[-700:]); buf.clear()
typ('find /lib/modules -name "*.ko*" | grep -E "ext4|ext3|jbd|crc16|mbcache|crc32c|libcrc" ; echo END_B')
pump('END_B\r?\n', 30); print(re.sub(r'\n\s*\n', '\n', buf.decode('utf8','replace'))[-700:]); buf.clear()
typ('ls /lib/modules/*/kernel/fs 2>&1 | head -30; echo END_C')
pump('END_C\r?\n', 30); print(re.sub(r'\n\s*\n', '\n', buf.decode('utf8','replace'))[-900:])
p.kill()