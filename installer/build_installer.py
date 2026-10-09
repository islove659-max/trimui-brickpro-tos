"""Dung bo cai dat TOS (cai tren may, khong can nap firmware).
Chay:  python -I -X utf8 build_installer.py   (can armhf_gl/build_runtime.ps1 truoc)
Dau ra ../out/pkg:
  TOS-Installer-<ver>.zip                  giai nen vao goc the SD, mo app "TOS - Cai dat" tren may
  TOS-Installer+PortMaster-<ver>.zip       nhu tren + goi PortMaster goc cho TrimUI (cho the moi)
  ../out/vm_sdroot/sdroot.tar              (de kiem thu may ao)
"""
import hashlib, io, json, os, sys, tarfile, zipfile
from PIL import Image, ImageDraw, ImageFont

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.abspath(os.path.join(HERE, '..'))
OUTD = os.path.join(ROOT, 'out', 'pkg'); os.makedirs(OUTD, exist_ok=True)
VER = '4.0'
ARMHF = r'C:\Users\X\tools\armhf_x'
GLR = os.path.join(ROOT, 'armhf_gl', 'dist')
PM_BASE = r'C:\Users\X\Downloads\trimui.portmaster.zip'
FONT = os.path.join(ROOT, 'sdk', 'rust', 'assets', 'BeVietnamPro-Regular.ttf')

def rd(p): return open(p, 'rb').read()
def lf(b): return b.replace(b'\r\n', b'\n').lstrip(b'\xef\xbb\xbf')

# ---- danh sach thanh phan: (dich, du lieu, mode) ----
files = []     # (dest, bytes, mode)
links = []     # (dest, target)
OV = os.path.join(ROOT, 'rootfs_overlay')
files += [
    ('/etc/udev/rules.d/61-trimui-input.rules', lf(rd(os.path.join(OV, 'etc', 'udev', 'rules.d', '61-trimui-input.rules'))), 0o644),
    ('/etc/sysctl.d/90-trimui-tuning.conf', lf(rd(os.path.join(OV, 'etc', 'sysctl.d', '90-trimui-tuning.conf'))), 0o644),
    ('/usr/trimui/bin/tos_gamemode.sh', lf(rd(os.path.join(OV, 'usr', 'trimui', 'bin', 'tos_gamemode.sh'))), 0o755),
    ('/usr/lib/tos/tos_env.sh', lf(rd(os.path.join(ROOT, 'sdk', 'shell', 'tos_env.sh'))), 0o644),
]
for it in json.load(open(os.path.join(ARMHF, 'manifest.json'), encoding='utf-8')):
    if it['type'] == 'file':
        data = rd(os.path.join(ARMHF, 'files', it['path'].lstrip('/').replace('/', os.sep)))
        assert hashlib.sha256(data).hexdigest() == it['sha256']
        files.append((it['path'], data, 0o755 if it['mode'] & 0o111 else 0o644))
    elif it['type'] == 'symlink':
        links.append((it['path'], it['target']))
for it in json.load(open(os.path.join(GLR, 'manifest.json'), encoding='utf-8')):
    if it['type'] == 'file':
        data = rd(os.path.join(GLR, 'files', it['path'].lstrip('/').replace('/', os.sep)))
        assert hashlib.sha256(data).hexdigest() == it['sha256']
        files.append((it['path'], data, it['mode']))
    elif it['type'] == 'symlink':
        links.append((it['path'], it['target']))

dirs = set()
for dest in [f[0] for f in files] + [l[0] for l in links]:
    d = os.path.dirname(dest)
    while d and d != '/':
        dirs.add(d); d = os.path.dirname(d)
dirs = sorted(dirs, key=lambda p: (p.count('/'), p))
for p in [f[0] for f in files] + [l[0] for l in links] + dirs:
    assert ' ' not in p, p

# ---- payload: MANIFEST + f/NNNN ----
sd = {}   # duong dan trong the SD -> (bytes, mode)
man = []
for d in dirs: man.append(f'D 755 {d}')
for i, (dest, data, mode) in enumerate(files):
    rel = f'f/{i:04d}'
    sd[f'System/tos/payload/{rel}'] = (data, 0o644)
    man.append(f'F {mode:o} {hashlib.sha256(data).hexdigest()} {len(data)} {rel} {dest}')
for dest, tgt in links: man.append(f'L {dest} {tgt}')
sd['System/tos/payload/MANIFEST'] = (('\n'.join(man) + '\n').encode(), 0o644)
sd['System/tos/payload/VERSION'] = ((VER + '\n').encode(), 0o644)
sd['System/tos/install.sh'] = (lf(rd(os.path.join(HERE, 'src', 'install.sh'))), 0o755)

# ---- app tren MainUI ----
def icon(text, color):
    im = Image.new('RGBA', (120, 120), (24, 28, 40, 255)); dr = ImageDraw.Draw(im)
    dr.rounded_rectangle((6, 6, 113, 113), 18, outline=color, width=5)
    f1 = ImageFont.truetype(FONT, 40); f2 = ImageFont.truetype(FONT, 17)
    dr.text((60, 48), 'TOS', font=f1, fill=color, anchor='mm'); dr.text((60, 88), text, font=f2, fill=(230, 230, 230), anchor='mm')
    b = io.BytesIO(); im.save(b, 'PNG'); return b.getvalue()
apps = {
    'TOS_CaiDat': ('TOS - Cai dat / Cap nhat', 'Cai runtime 32-bit, glremote va toi uu he thong (che do game). Khong xoa du lieu.', 'install', 'CAI DAT', (80, 200, 120)),
    'TOS_GoCaiDat': ('TOS - Go cai dat', 'Go sach TOS, tra lai tep goc.', 'uninstall', 'GO', (220, 110, 90)),
}
for name, (label, desc, action, ictext, color) in apps.items():
    cfg = json.dumps({'label': label, 'icon': 'icon.png', 'iconsel': 'icon.png', 'icontop': 'icon.png', 'launch': 'launch.sh', 'description': desc, 'themecolor': '50C878'}, indent=2)
    sd[f'Apps/{name}/config.json'] = (cfg.encode('ascii'), 0o644)
    sd[f'Apps/{name}/launch.sh'] = (f'#!/bin/sh\ncd "$(dirname "$0")"\nexec sh /mnt/SDCARD/System/tos/install.sh {action}\n'.encode(), 0o755)
    sd[f'Apps/{name}/icon.png'] = (icon(ictext, color), 0o644)

HD = f"""TOS {VER} - bo cai dat 1 lan cho TrimUI Brick Pro (Stock OS)
================================================================
Gom: runtime 32-bit (ARMHF) + glremote (chay game 32-bit bang GPU that, co am thanh) + toi uu he thong
(che do game: tu tam tat dich vu nen khi vao game; sua nhan tay cau/thiet bi cho port PortMaster; tinh chinh bo nho).

CAI DAT
  1. Giai nen file zip vao GOC the SD (thu muc Apps, System se gop vao). Khong ghi de file nao cua ban.
  2. Tren may mo muc Apps -> "TOS - Cai dat / Cap nhat". Man hinh bao tien trinh; xong thi KHOI DONG LAI may.
  3. Game 32-bit trong PortMaster: trong script port dung  glremote_run ./TenGame.armhf   (xem HUONG_DAN trong repo GitHub).
CAP NHAT: giai nen ban moi roi chay lai "TOS - Cai dat". GO BO: Apps -> "TOS - Go cai dat" (tra lai tep goc, khong mat du lieu).

AN TOAN
  * Chi chep tep vao tang ghi cua he thong (overlay) va chen 2 dong vao preload.sh/premainui.sh (co ban sao luu tai
    /mnt/UDISK/tos/backup). KHONG nap lai firmware, KHONG dung toi bootloader/nhan/the game. Nhat ky: System/tos/install.log.
  * Chua ap dung: loglevel=4 cua nhan (chi co trong ban firmware nap day du).
  * San pham ca nhan, khong lien quan TrimUI. Tu chiu rui ro khi cai.
Ma nguon: https://github.com/islove659-max/trimui-brickpro-tos
"""
NOTICE = """Thanh phan ben thu ba:
- Thu vien 32-bit Debian 11 (glibc 2.31, libgcc-s1, libstdc++6; LGPL) lay tu deb.debian.org; ma nguon: packages.debian.org (bullseye).
- SDL2 2.0.14 Debian 11 (zlib). PortMaster (MIT; chi co trong ban kem PortMaster, khong sua).
Phan mem hang TrimUI khong duoc dua kem.
"""
sd['HUONG_DAN_TOS.txt'] = (HD.encode('utf-8'), 0o644)
sd['System/tos/NOTICE.txt'] = (NOTICE.encode('utf-8'), 0o644)

def zip_write(zf, name, data, mode):
    zi = zipfile.ZipInfo(name, date_time=(2026, 10, 9, 0, 0, 0)); zi.compress_type = zipfile.ZIP_DEFLATED
    zi.external_attr = (0o100000 | mode) << 16; zf.writestr(zi, data)

a = os.path.join(OUTD, f'TOS-Installer-{VER}.zip')
with zipfile.ZipFile(a, 'w') as zf:
    for n in sorted(sd): zip_write(zf, n, *sd[n])
b = os.path.join(OUTD, f'TOS-Installer+PortMaster-{VER}.zip')
with zipfile.ZipFile(PM_BASE) as base, zipfile.ZipFile(b, 'w') as zf:
    names = set()
    for zi in base.infolist():
        zf.writestr(zi, base.read(zi.filename), compress_type=zi.compress_type); names.add(zi.filename)
    assert not [n for n in sd if n in names], 'trung ten voi goi PortMaster'
    for n in sorted(sd): zip_write(zf, n, *sd[n])

vm = os.path.join(ROOT, 'out', 'vm_sdroot'); os.makedirs(vm, exist_ok=True)
with tarfile.open(os.path.join(vm, 'sdroot.tar'), 'w', format=tarfile.USTAR_FORMAT) as tf:
    for n in sorted(sd):
        ti = tarfile.TarInfo(n); ti.size = len(sd[n][0]); ti.mode = sd[n][1]; tf.addfile(ti, io.BytesIO(sd[n][0]))

for p in (a, b):
    h = hashlib.sha256(rd(p)).hexdigest(); open(p + '.sha256', 'w').write(f'{h}  {os.path.basename(p)}\n')
    print(f'{os.path.basename(p)}: {os.path.getsize(p)/1e6:.2f} MB sha256 {h[:16]}...')
print('payload:', len(files), 'tep,', len(links), 'symlink,', len(dirs), 'thu muc')
