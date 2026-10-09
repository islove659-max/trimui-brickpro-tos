"""Dựng thẻ cứu hộ "TOS v2" cho TrimUI Brick Pro (TG4040) từ thẻ cứu hộ gốc 1.1.1-20260717.

Chỉ chứa các thay đổi ĐÃ KIỂM CHỨNG trên máy thật (xem docs/ và phan_tich_mainui/):
  1. luật udev gắn ID_INPUT* cho thiết bị input (libinput/Weston/SDL thấy tay cầm)
  2. symlink /run -> /var/run
  3. sysctl nhẹ (printk, inotify, rmem/wmem, vfs_cache_pressure)
  4. "chế độ game": tos_gamemode.sh + móc vào preload.sh / premainui.sh (có công tắc GAMEMODE=0)
  5. thư viện nền TOS cho launcher: /usr/lib/tos/tos_env.sh
  6. (tuỳ chọn) env u-boot: loglevel 8 -> 4
KHÔNG có: swap, bỏ sync thẻ SD, đổi fstab, kernel.panic (bài học bản v1 gây reset + hỏng FAT).

Cách chạy:  python -I -X utf8 build_fw_v2.py            (thêm --khong-loglevel để giữ loglevel=8)
Cách làm: sửa thẳng trên ảnh ext4 bằng ext4lite (không đóng gói lại thư mục, vì Windows làm mất symlink/quyền).
Kích thước mọi phần trong gói IMAGEWTY giữ nguyên; chỉ ghi đè rootfs/env tại chỗ rồi tính lại checksum V*.fex.
"""
import hashlib, os, shutil, struct, sys, time, zlib

OLD_TOOLS = r'C:\Users\X\dự án chỉnh sửa lại flimware\tools'
OLD_ROOT = r'C:\Users\X\dự án chỉnh sửa lại flimware'
sys.path.insert(0, OLD_TOOLS)
import build_firmware as bf                      # tái dùng addsum, imagewty_table, replace_once, build_env
from ext4lite import Ext4

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.abspath(os.path.join(HERE, '..'))
ORIG_IMG = r'D:\sd_recovery_tg4040_brick_pro_v1.1.1_20260717\sd_recovery_tg4040_brickpro_ver1.1.1_20260717.img'
ROOTFS_SRC = os.path.join(OLD_ROOT, 'goc', 'fex', 'rootfs.fex')
OVERLAY = os.path.join(ROOT, 'rootfs_overlay')
SDK_SHELL = os.path.join(ROOT, 'sdk', 'shell')
OUTDIR = os.path.join(ROOT, 'out', 'fw_v2')
BUILD = os.path.join(OUTDIR, 'build')
OUT_IMG = os.path.join(OUTDIR, 'sd_recovery_tg4040_brickpro_ver1.1.1_20260717_tos2.img')
VERSION = 'tos-2.0'
IMG_BASE = bf.IMG_BASE

LOGLEVEL = '--khong-loglevel' not in sys.argv

# --armhf <thu_muc>: thêm runtime ARMHF 32-bit (kết quả của lay_armhf.py) => bản tos-3.0 ở out/fw_v3
ARMHF = sys.argv[sys.argv.index('--armhf') + 1] if '--armhf' in sys.argv else None
if ARMHF:
    OUTDIR = os.path.join(ROOT, 'out', 'fw_v3')
    BUILD = os.path.join(OUTDIR, 'build')
    OUT_IMG = os.path.join(OUTDIR, 'sd_recovery_tg4040_brickpro_ver1.1.1_20260717_tos3.img')
    VERSION = 'tos-3.0'

# --glremote <thu_muc>: thêm runtime glremote (GPU + âm thanh cho ARMHF; kết quả armhf_gl/build_runtime.ps1) => tos-4.0 ở out/fw_v4 (cần --armhf)
GLR = sys.argv[sys.argv.index('--glremote') + 1] if '--glremote' in sys.argv else None
if GLR:
    assert ARMHF, '--glremote cần --armhf'
    OUTDIR = os.path.join(ROOT, 'out', 'fw_v4')
    BUILD = os.path.join(OUTDIR, 'build')
    OUT_IMG = os.path.join(OUTDIR, 'sd_recovery_tg4040_brickpro_ver1.1.1_20260717_tos4.img')
    VERSION = 'tos-4.0'

# (đường dẫn trong rootfs, nguồn trên PC, quyền)
NEW_FILES = [
    ('/etc/udev/rules.d/61-trimui-input.rules', os.path.join(OVERLAY, 'etc', 'udev', 'rules.d', '61-trimui-input.rules'), 0o644),
    ('/etc/sysctl.d/90-trimui-tuning.conf', os.path.join(OVERLAY, 'etc', 'sysctl.d', '90-trimui-tuning.conf'), 0o644),
    ('/usr/trimui/bin/tos_gamemode.sh', os.path.join(OVERLAY, 'usr', 'trimui', 'bin', 'tos_gamemode.sh'), 0o755),
    ('/usr/lib/tos/tos_env.sh', os.path.join(SDK_SHELL, 'tos_env.sh'), 0o644),
]
NEW_DIRS = ['/usr/lib/tos']

# vá file có sẵn: mẫu khớp đúng 1 lần
PATCHES = {
    '/usr/trimui/bin/preload.sh': [
        ("rm /tmp/stay_awake\n",
         "rm /tmp/stay_awake\n"
         "\n"
         "# [tos] chay sau khi MainUI thoat, truoc khi app/game chay: chi bat che do game khi sap mo app\n"
         "[ -f /tmp/cmd_to_run.sh ] && [ -x /usr/trimui/bin/tos_gamemode.sh ] && /usr/trimui/bin/tos_gamemode.sh on\n"),
    ],
    '/usr/trimui/bin/premainui.sh': [
        ("rm -f /tmp/trimui_inputd/input_dpad_to_joystick\n",
         "rm -f /tmp/trimui_inputd/input_dpad_to_joystick\n"
         "\n"
         "# [tos] truoc khi MainUI chay lai: bat lai dich vu da dung trong che do game\n"
         "[ -x /usr/trimui/bin/tos_gamemode.sh ] && /usr/trimui/bin/tos_gamemode.sh off\n"),
    ],
}


def read_text_lf(path):
    data = open(path, 'rb').read()
    if data.startswith(b'\xef\xbb\xbf'): data = data[3:]
    data = data.replace(b'\r\n', b'\n')
    assert b'\r' not in data, path
    return data


def build_rootfs():
    os.makedirs(BUILD, exist_ok=True)
    dst = os.path.join(BUILD, 'rootfs.fex')
    shutil.copyfile(ROOTFS_SRC, dst)
    e = Ext4(dst, write=True)
    free0 = e.free_blocks
    report = []

    # 1. vá file có sẵn
    for path, pats in PATCHES.items():
        ino = e.lookup(path)
        assert ino, f'không có {path} trong rootfs gốc'
        raw = e.read_inode(ino)
        text = e.read_data(ino).decode('utf-8')
        assert '[tos]' not in text, f'{path} đã có móc [tos] (rootfs không phải bản gốc?)'
        for old, new in pats:
            text = bf.replace_once(text, old, new, path)
        e.write_file(path, text.encode('utf-8'))
        assert e.read_inode(ino)[0:2] == raw[0:2], f'{path}: quyền file bị đổi'
        report.append(f'vá  {path}')

    # 2. thư mục mới
    for d in NEW_DIRS:
        if e.lookup(d) is None: report.append(f'tạo {d}/')
        e.mkdir(d, 0o755)

    # 3. file mới (không được tồn tại trong rootfs gốc)
    for path, src, mode in NEW_FILES:
        data = read_text_lf(src)
        assert e.lookup(path) is None, f'{path} đã có trong rootfs gốc - phải vá bằng PATCHES'
        e.write_file(path, data, mode)
        report.append(f'thêm {path} ({len(data)} B, {oct(mode)})')

    # 4. /run -> /var/run
    assert e.lookup('/run') is None, '/run đã tồn tại'
    e.symlink('/run', '/var/run')
    report.append('thêm symlink /run -> /var/run')

    # 4b. runtime ARMHF 32-bit (Debian 11: glibc 2.31, libgcc_s, libstdc++ 6.0.28)
    if ARMHF:
        import json
        man = json.load(open(os.path.join(ARMHF, 'manifest.json'), encoding='utf-8'))
        for d in ('/lib/arm-linux-gnueabihf', '/usr/lib/arm-linux-gnueabihf'):
            assert e.lookup(d) is None, f'{d} đã tồn tại trong rootfs gốc'
            e.mkdir(d, 0o755); report.append(f'tạo {d}/')
        for it in man:
            if it['type'] != 'file': continue
            assert e.lookup(it['path']) is None, f'{it["path"]} đã có'
            data = open(os.path.join(ARMHF, 'files', it['path'].lstrip('/').replace('/', os.sep)), 'rb').read()
            assert hashlib.sha256(data).hexdigest() == it['sha256'], f'sha256 sai: {it["path"]}'
            e.write_file(it['path'], data, 0o755 if it['mode'] & 0o111 else 0o644)
        nl = 0
        for it in man:
            if it['type'] != 'symlink': continue
            assert e.lookup(it['path']) is None, f'{it["path"]} đã có'
            e.symlink(it['path'], it['target']); nl += 1
        report.append(f'ARMHF: {sum(1 for i in man if i["type"]=="file")} file + {nl} symlink (loader /lib/ld-linux-armhf.so.3)')

    # 4c. runtime glremote (GPU/âm thanh cho ARMHF)
    if GLR:
        import json
        gman = json.load(open(os.path.join(GLR, 'manifest.json'), encoding='utf-8'))
        for it in gman:
            if it['type'] == 'dir':
                assert e.lookup(it['path']) is None, f'{it["path"]} đã tồn tại'
                e.mkdir(it['path'], 0o755)
        for it in gman:
            if it['type'] != 'file': continue
            assert e.lookup(it['path']) is None, f'{it["path"]} đã có'
            data = open(os.path.join(GLR, 'files', it['path'].lstrip('/').replace('/', os.sep)), 'rb').read()
            assert hashlib.sha256(data).hexdigest() == it['sha256'], f'sha256 sai: {it["path"]}'
            e.write_file(it['path'], data, it['mode'])
        for it in gman:
            if it['type'] == 'symlink':
                assert e.lookup(it['path']) is None, f'{it["path"]} đã có'
                e.symlink(it['path'], it['target'])
        report.append(f'glremote: {sum(1 for i in gman if i["type"] == "file")} file + {sum(1 for i in gman if i["type"] == "symlink")} symlink (/usr/lib/glremote, /usr/bin/glremote_run)')

    # 5. dấu phiên bản
    stamp = (f'{VERSION}\nbase: tg4040 brickpro 1.1.1-20260717\nbuild: {time.strftime("%Y-%m-%d %H:%M")}\n'
             f'loglevel4: {1 if LOGLEVEL else 0}\narmhf: {1 if ARMHF else 0}\nglremote: {1 if GLR else 0}\n').encode()
    e.write_file('/etc/tos_version', stamp, 0o644)
    report.append('thêm /etc/tos_version')

    e.flush()
    errs = e.fsck()
    assert not errs, 'fsck lỗi: ' + '; '.join(errs[:10])
    used = (free0 - e.free_blocks) * e.bs
    report.append(f'rootfs: dùng thêm {used/1e3:.1f} KB, còn trống {e.free_blocks*e.bs/1e6:.1f} MB, fsck OK')

    # đọc lại để chắc chắn nội dung ghi đúng
    for path, src, mode in NEW_FILES:
        assert e.read_file(path) == read_text_lf(src), f'đọc lại {path} không khớp'
    for path in PATCHES:
        assert '[tos]' in e.read_file(path).decode()
    assert e.lookup('/run') is not None
    if ARMHF:
        for it in man:
            if it['type'] == 'file':
                assert hashlib.sha256(e.read_file(it['path'])).hexdigest() == it['sha256'], f'đọc lại {it["path"]} không khớp'
            else:
                ino = e.lookup(it['path'])
                assert ino is not None, f'thiếu symlink {it["path"]}'
        assert e.lookup('/lib/ld-linux-armhf.so.3') is not None
    if GLR:
        for it in gman:
            if it['type'] == 'file':
                assert hashlib.sha256(e.read_file(it['path'])).hexdigest() == it['sha256'], f'đọc lại {it["path"]} không khớp'
            elif it['type'] == 'symlink':
                assert e.lookup(it['path']) is not None, f'thiếu {it["path"]}'
    e.close()
    return dst, report


def main():
    os.makedirs(OUTDIR, exist_ok=True)
    print('== rootfs')
    rootfs_path, report = build_rootfs()
    for r in report: print('  ', r)
    rootfs = open(rootfs_path, 'rb').read()

    with open(ORIG_IMG, 'rb') as f:
        tab = bf.imagewty_table(f)
        by = {}
        for t in tab: by.setdefault(t['name'], []).append(t)
        f.seek(IMG_BASE + by['env.fex'][0]['off']); env_orig = f.read(by['env.fex'][0]['orig'])

    assert len(rootfs) == by['rootfs.fex'][0]['orig'], 'kích thước rootfs đổi - không vá tại chỗ được'
    writes = [(by['rootfs.fex'][0]['off'], rootfs),
              (by['Vrootfs.fex'][0]['off'], struct.pack('<I', bf.addsum(rootfs)))]
    env_new = env_orig
    if LOGLEVEL:
        bf.ENV_PATCHES = [('loglevel=8\0', 'loglevel=4\0')]
        env_new = bf.build_env(env_orig)
        for t in by['env.fex']: writes.append((t['off'], env_new))
        for t in by['Venv.fex']: writes.append((t['off'], struct.pack('<I', bf.addsum(env_new))))
    else:
        print('   (giữ nguyên env u-boot: loglevel=8)')

    print('== chép thẻ gốc -> out (2.5GB)')
    tmp = OUT_IMG + '.tmp'
    shutil.copyfile(ORIG_IMG, tmp)
    with open(tmp, 'r+b') as f:
        for off, data in writes:
            f.seek(IMG_BASE + off); f.write(data)
    os.replace(tmp, OUT_IMG)

    print('== kiểm tra thẻ mới')
    with open(OUT_IMG, 'rb') as f:
        tab2 = bf.imagewty_table(f)
        assert [(t['name'], t['off'], t['orig']) for t in tab2] == [(t['name'], t['off'], t['orig']) for t in tab], 'bảng IMAGEWTY đổi'
        names = [t['name'] for t in tab2]
        for t in tab2:
            if t['name'].startswith('V') and t['name'][1:] in names:
                dd = [x for x in tab2 if x['name'] == t['name'][1:] and x['i'] == t['i'] - 1]
                if not dd: continue
                d = dd[0]
                f.seek(IMG_BASE + d['off']); data = f.read(d['orig'])
                f.seek(IMG_BASE + t['off']); v = struct.unpack('<I', f.read(4))[0]
                ok = bf.addsum(data) == v
                print(f'   {d["name"]:20} checksum {"OK" if ok else "SAI"}')
                assert ok
        f.seek(IMG_BASE + by['rootfs.fex'][0]['off'])
        assert hashlib.sha256(f.read(len(rootfs))).digest() == hashlib.sha256(rootfs).digest()
        f.seek(IMG_BASE + by['env.fex'][0]['off']); e2 = f.read(len(env_new))
        assert struct.unpack('<I', e2[:4])[0] == zlib.crc32(e2[5:]), 'CRC env sai'
        if LOGLEVEL: assert b'loglevel=4\0' in e2
        print('   env: CRC OK' + (', loglevel=4' if LOGLEVEL else ''))

    # so từng byte với thẻ gốc: chỉ rootfs/Vrootfs/env/Venv được đổi
    allowed = []
    for off, data in writes: allowed.append((IMG_BASE + off, IMG_BASE + off + len(data)))
    print('== so sánh từng byte với thẻ gốc (chỉ vùng đã ghi được phép khác)')
    diff_outside = 0
    with open(ORIG_IMG, 'rb') as a, open(OUT_IMG, 'rb') as b:
        pos = 0
        CH = 1 << 22
        while True:
            x = a.read(CH); y = b.read(CH)
            if not x and not y: break
            if x != y:
                # tìm đoạn khác trong chunk, đối chiếu vùng cho phép
                for i in range(0, len(x), 4096):
                    if x[i:i+4096] != y[i:i+4096]:
                        p = pos + i
                        # khối 4KB [p, p+4096) giao với vùng được phép (vùng 4 byte V*.fex không thẳng hàng 4KB)
                        if not any(p < e_ and p + 4096 > s for s, e_ in allowed):
                            diff_outside += 1; print('   KHAC NGOAI VUNG tai offset', hex(p))
            pos += len(x)
    print('   khối 4KB khác NGOÀI vùng đã ghi:', diff_outside)
    assert diff_outside == 0

    h = hashlib.sha256()
    with open(OUT_IMG, 'rb') as f:
        for c in iter(lambda: f.read(1 << 24), b''): h.update(c)
    open(OUT_IMG + '.sha256', 'w').write(f'{h.hexdigest()}  {os.path.basename(OUT_IMG)}\n')
    with open(os.path.join(OUTDIR, 'BAO_CAO_BUILD.txt'), 'w', encoding='utf-8') as r:
        r.write(f'{VERSION} build {time.strftime("%Y-%m-%d %H:%M")}\nloglevel4={LOGLEVEL}\n' + '\n'.join(report) + f'\nsha256 {h.hexdigest()}\n')
    print('== xong:', OUT_IMG)
    print('   sha256', h.hexdigest())


if __name__ == '__main__':
    main()
