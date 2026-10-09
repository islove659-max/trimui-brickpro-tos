"""Dong goi glremote cho THE SD / PortMaster (FAT32 khong co symlink => dung ban sao).
Dung:  python -I -X utf8 build_sd_pack.py            (can chay armhf_gl/build_runtime.ps1 truoc)
Dau ra (thu muc ../out/pkg):
  glremote-sd-pack-<ver>.zip                       giai nen vao goc the SD (System/bin/glremote_run + System/glremote/...)
  TrimUI PortMaster + glremote (Brick Pro) <ver>.zip   goi PortMaster goc cho TrimUI + glremote (cai lan dau la co san)
  vm_extra/sdpack.tar                              (de kiem thu may ao)
"""
import hashlib, io, json, os, sys, tarfile, zipfile

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.abspath(os.path.join(HERE, '..'))
DIST = os.path.join(HERE, 'dist')
STAGE = os.path.join(DIST, 'stage')
OUTD = os.path.join(ROOT, 'out', 'pkg')
VER = '0.1.0'
PM_BASE = r'C:\Users\X\Downloads\trimui.portmaster.zip'    # goi PortMaster goc cho TrimUI (tai tu GitHub), KHONG sua file nao trong do
os.makedirs(OUTD, exist_ok=True)

man = json.load(open(os.path.join(DIST, 'manifest.json'), encoding='utf-8'))
R = '/usr/lib/glremote/'
files = {}   # duong dan trong the SD -> bytes

def rd(p): return open(p, 'rb').read()
src_of = {m['path']: rd(os.path.join(DIST, 'files', m['path'].lstrip('/').replace('/', os.sep))) for m in man if m['type'] == 'file'}
for m in man:
    if m['type'] == 'file' and m['path'].startswith(R):
        files['System/glremote/' + m['path'][len(R):]] = src_of[m['path']]
for m in man:
    if m['type'] == 'symlink':      # the SD la FAT32: thay symlink bang ban sao cung noi dung
        tgt = os.path.normpath(os.path.join(os.path.dirname(m['path']), m['target'])).replace('\\', '/')
        files['System/glremote/' + m['path'][len(R):]] = src_of[tgt]
glr_run = src_of['/usr/bin/glremote_run']
files['System/bin/glremote_run'] = glr_run
files['System/glremote/VERSION'] = (VER + '\n').encode()

HUONG_DAN = f"""glremote {VER} - GPU + am thanh cho game ARMHF (32-bit) tren TrimUI Brick Pro
=====================================================================================
Yeu cau: firmware co runtime ARMHF (tos-3.0 tro len, co /lib/ld-linux-armhf.so.3). Firmware tos-4.0 da co san glremote
trong /usr/lib/glremote - goi nay chi can cho tos-3.0 hoac khi muon cap nhat rieng glremote tren the SD.

Cai: giai nen vao GOC the SD (se co System/bin/glremote_run va thu muc System/glremote). Khong ghi de file nao khac.

Dung trong script cua port PortMaster chi co ban ARMHF (thay cho dong chay game):
    export GAMELIBS="$GAMEDIR/libs.armhf"          # thu vien 32-bit cua game (neu co, uu tien truoc)
    /mnt/SDCARD/System/bin/glremote_run ./TenGame.armhf
(Neu firmware co san thi lenh "glremote_run" nam trong PATH; script uu tien /usr/lib/glremote, khong thi System/glremote.)

Bien moi truong (tuy chon):  GAMELIBS, GLR_MAX (gioi han giay), GLR_NULL=1 (thu khong man hinh/loa),
SDL_VIDEODRIVER (mac dinh KMSDRM_LEGACY), SDL_AUDIODRIVER (mac dinh alsa) - ca hai la ban gia do glremote cung cap.

Gioi han: chi OpenGL ES 2 (khong GLES3/VAO), mot luong GL, am thanh S16 48 kHz, chua co ghi am.
Loi da biet: SDL2 32-bit du phong (Debian 11) segfault khi SDL_QuitSubSystem(JOYSTICK) luc thoat; game mang SDL2 rieng khong bi.
Go bo: xoa System/bin/glremote_run va thu muc System/glremote.
"""
files['System/glremote/HUONG_DAN.txt'] = HUONG_DAN.encode('utf-8')

def zip_write(zf, name, data, mode=0o755):
    zi = zipfile.ZipInfo(name, date_time=(2026, 10, 9, 0, 0, 0))
    zi.compress_type = zipfile.ZIP_DEFLATED
    zi.external_attr = (0o100000 | mode) << 16
    zf.writestr(zi, data)

pack = os.path.join(OUTD, f'glremote-sd-pack-{VER}.zip')
with zipfile.ZipFile(pack, 'w') as zf:
    zip_write(zf, 'HUONG_DAN.txt', HUONG_DAN.encode('utf-8'), 0o644)
    for n in sorted(files): zip_write(zf, n, files[n], 0o644 if n.endswith(('.txt', 'VERSION')) else 0o755)

combo = os.path.join(OUTD, f'TrimUI PortMaster + glremote (Brick Pro) {VER}.zip')
with zipfile.ZipFile(PM_BASE) as base, zipfile.ZipFile(combo, 'w') as zf:
    names = set()
    for zi in base.infolist():
        data = base.read(zi.filename)
        zf.writestr(zi, data, compress_type=zi.compress_type)
        names.add(zi.filename)
    clash = [n for n in files if n in names]
    assert not clash, f'trung ten voi goi PortMaster goc: {clash[:3]}'
    zip_write(zf, 'HUONG_DAN_GLREMOTE.txt', HUONG_DAN.encode('utf-8'), 0o644)
    for n in sorted(files): zip_write(zf, n, files[n], 0o644 if n.endswith(('.txt', 'VERSION')) else 0o755)

# tar cho may ao
vm = os.path.join(ROOT, 'out', 'vm_sdpack'); os.makedirs(vm, exist_ok=True)
with tarfile.open(os.path.join(vm, 'sdpack.tar'), 'w', format=tarfile.USTAR_FORMAT) as tf:
    for n in sorted(files):
        ti = tarfile.TarInfo(n); ti.size = len(files[n]); ti.mode = 0o755; tf.addfile(ti, io.BytesIO(files[n]))

for p in (pack, combo):
    h = hashlib.sha256(open(p, 'rb').read()).hexdigest()
    open(p + '.sha256', 'w').write(f'{h}  {os.path.basename(p)}\n')
    print(f'{os.path.basename(p)}: {os.path.getsize(p)/1e6:.2f} MB sha256 {h[:16]}...')
print('trong goi:', len(files), 'tep')
