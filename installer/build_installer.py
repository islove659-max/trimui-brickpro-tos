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
# hinh nen + font cho thong bao tren man hinh (khong phu thuoc System/resources cua the)
sd['System/tos/res/bg.png'] = (rd(os.path.join(HERE, 'res', 'bg.png')), 0o644)
sd['System/tos/res/font.ttf'] = (rd(os.path.join(HERE, 'res', 'font.ttf')), 0o644)
sd['System/tos/res/OFL.txt'] = (rd(os.path.join(ROOT, 'sdk', 'rust', 'assets', 'OFL.txt')), 0o644)

# ---- anh thong bao tren man hinh (co dau tieng Viet; sdl2imgshow chi ve chu Latin-1 nen ve san thanh anh) ----
def msg_png(title, sub, accent):
    W, H = 1024, 768
    im = Image.new('RGB', (W, H)); dr = ImageDraw.Draw(im)
    for y in range(H):
        c = int(18 + (y / H) * 22); dr.line([(0, y), (W, y)], fill=(c, c + 6, c + 20))
    ft = ImageFont.truetype(FONT, 54); fs = ImageFont.truetype(FONT, 30)
    dr.rounded_rectangle((70, 250, W - 70, 520), 28, outline=accent, width=5)
    dr.text((W // 2, 345), title, font=ft, fill=(255, 255, 255), anchor='mm')
    lines, cur = [], ''
    for w in sub.split():
        t = (cur + ' ' + w).strip()
        if dr.textlength(t, font=fs) > W - 200: lines.append(cur); cur = w
        else: cur = t
    if cur: lines.append(cur)
    for i, ln in enumerate(lines): dr.text((W // 2, 430 + i * 42), ln, font=fs, fill=(205, 215, 230), anchor='mm')
    b = io.BytesIO(); im.save(b, 'PNG', optimize=True); return b.getvalue()
GREEN, RED, YEL = (80, 200, 120), (230, 100, 90), (240, 190, 80)
MSGS = {
    'start': ('Đang cài đặt TOS 4.0...', 'Vui lòng chờ, đừng tắt máy.', GREEN),
    'copy': ('Đang chép tệp hệ thống...', 'Chỉ mất vài giây.', GREEN),
    'done': ('Cài đặt xong!', 'Hãy KHỞI ĐỘNG LẠI máy để áp dụng đầy đủ.', GREEN),
    'un_start': ('Đang gỡ cài đặt TOS...', 'Vui lòng chờ, đừng tắt máy.', YEL),
    'un_done': ('Gỡ cài đặt xong.', 'Hãy khởi động lại máy.', YEL),
    'un_none': ('TOS chưa được cài.', 'Không có gì để gỡ.', YEL),
    'err_arch': ('Lỗi: máy không phù hợp', 'Bộ cài chỉ dành cho TrimUI Brick Pro (aarch64).', RED),
    'err_stock': ('Lỗi: không phải Stock OS TrimUI', 'Không tìm thấy các tệp khởi động của hệ điều hành gốc.', RED),
    'err_payload': ('Lỗi: thiếu gói cài đặt', 'Hãy giải nén ĐẦY ĐỦ file zip vào gốc thẻ SD rồi thử lại.', RED),
    'err_hash': ('Lỗi: tệp trong gói bị hỏng', 'Hãy tải lại gói cài đặt rồi giải nén lại.', RED),
    'err_space': ('Lỗi: hết chỗ trống trong hệ thống', 'Cần ít nhất 15 MB trống.', RED),
    'err_write': ('Lỗi: không ghi được tệp hệ thống', 'Xem nhật ký System/tos/install.log trên thẻ SD.', RED),
    'err': ('Lỗi khi cài đặt', 'Xem nhật ký System/tos/install.log trên thẻ SD.', RED),
}
for k, (a, b, c) in MSGS.items():
    sd[f'System/tos/res/m_{k}.png'] = (msg_png(a, b, c), 0o644)

# ---- app tren MainUI ----
def icon(text, color):
    # 300x300 RGBA, nen trong suot, o vuong bo goc o phia tren (cung kich thuoc/kieu icon PortMaster tren the)
    im = Image.new('RGBA', (300, 300), (0, 0, 0, 0)); dr = ImageDraw.Draw(im)
    tile = Image.new('RGBA', (176, 172), (0, 0, 0, 0)); td = ImageDraw.Draw(tile)
    for y in range(172):
        k = y / 171.0
        td.line([(0, y), (176, y)], fill=(int(24 + 18 * k), int(30 + 22 * k), int(48 + 30 * k), 255))
    mask = Image.new('L', (176, 172), 0); ImageDraw.Draw(mask).rounded_rectangle((0, 0, 175, 171), 40, fill=255)
    im.paste(tile, (62, 24), mask)
    dr.rounded_rectangle((62, 24, 237, 195), 40, outline=color, width=6)
    f1 = ImageFont.truetype(FONT, 62); f2 = ImageFont.truetype(FONT, 26)
    dr.text((150, 92), 'TOS', font=f1, fill=color, anchor='mm'); dr.text((150, 158), text, font=f2, fill=(235, 235, 235), anchor='mm')
    b = io.BytesIO(); im.save(b, 'PNG'); return b.getvalue()
apps = {
    'TOS_CaiDat': ('TOS - Cai dat / Cap nhat', 'Cai runtime 32-bit, glremote va toi uu he thong (che do game). Khong xoa du lieu.', 'install', 'CAI DAT', (80, 200, 120)),
    'TOS_GoCaiDat': ('TOS - Go cai dat', 'Go sach TOS, tra lai tep goc.', 'uninstall', 'GO', (220, 110, 90)),
}
for name, (label, desc, action, ictext, color) in apps.items():
    # chi 'icon' + 'iconsel' (giong PortMaster); KHONG them 'icontop' vi MainUI ve ca hai -> 2 anh de len nhau
    cfg = json.dumps({'label': label, 'icon': 'icon.png', 'iconsel': 'icon.png', 'launch': 'launch.sh', 'description': desc}, indent=2)
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
