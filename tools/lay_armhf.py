"""Giải nén 3 gói Debian armhf (libc6, libgcc-s1, libstdc++6) và chọn các file tối thiểu cho runtime ARMHF 32-bit.
Chỉ ĐỌC các gói đã tải (ar + data.tar.xz bằng Python thuần), KHÔNG chạy gì trong đó.
Kết quả: <out>/files/<đường dẫn> (file thường) và <out>/manifest.json (file + symlink + quyền), dùng cho build_fw_v3.py.
Dùng: python -I -X utf8 lay_armhf.py <thu_muc_deb> <thu_muc_ra> [--liet-ke]"""
import hashlib, io, json, lzma, os, sys, tarfile

DEB_DIR, OUT = sys.argv[1], sys.argv[2]
LIST_ONLY = '--liet-ke' in sys.argv
DEBS = ['libc6_2.31-13+deb11u11_armhf.deb', 'libgcc-s1_10.2.1-6_armhf.deb', 'libstdc++6_10.2.1-6_armhf.deb']

# Chỉ lấy thư viện thời chạy (không lấy locale, gconv, tài liệu, công cụ).
KEEP_PREFIX = ('lib/arm-linux-gnueabihf/', 'usr/lib/arm-linux-gnueabihf/', 'lib/ld-linux-armhf.so.3')
KEEP_NAMES = ('ld-', 'libc-', 'libc.so', 'libm-', 'libm.so', 'libpthread', 'libdl', 'librt', 'libutil', 'libresolv',
              'libgcc_s', 'libstdc++', 'libnss_files', 'libnss_dns', 'libanl', 'libnsl', 'libBrokenLocale')


def ar_members(data):
    assert data[:8] == b'!<arch>\n'
    i = 8
    while i + 60 <= len(data):
        name = data[i:i + 16].decode().strip().rstrip('/')
        size = int(data[i + 48:i + 58].decode().strip())
        yield name, data[i + 60:i + 60 + size]
        i += 60 + size + (size & 1)


def data_tar(deb_bytes):
    for name, body in ar_members(deb_bytes):
        if name.startswith('data.tar'):
            if name.endswith('.xz'): body = lzma.decompress(body)
            elif name.endswith('.gz'):
                import gzip; body = gzip.decompress(body)
            return tarfile.open(fileobj=io.BytesIO(body))
    raise SystemExit('khong thay data.tar trong ' )

manifest = []
for deb in DEBS:
    raw = open(os.path.join(DEB_DIR, deb), 'rb').read()
    tf = data_tar(raw)
    print(f'== {deb}: {len(raw):,} byte')
    for m in tf.getmembers():
        p = m.name.lstrip('./')
        if m.isdir(): continue
        base = os.path.basename(p)
        keep = p.startswith(KEEP_PREFIX) and base.startswith(KEEP_NAMES)
        if LIST_ONLY:
            print('   %s %-62s %9d %s' % ('KEEP' if keep else '    ', p, m.size, ('-> ' + m.linkname) if m.issym() else ''))
            continue
        if not keep: continue
        if m.issym():
            manifest.append({'path': '/' + p, 'type': 'symlink', 'target': m.linkname})
        elif m.isfile():
            data = tf.extractfile(m).read()
            dst = os.path.join(OUT, 'files', p.replace('/', os.sep))
            os.makedirs(os.path.dirname(dst), exist_ok=True)
            open(dst, 'wb').write(data)
            manifest.append({'path': '/' + p, 'type': 'file', 'mode': m.mode & 0o7777, 'size': len(data),
                             'sha256': hashlib.sha256(data).hexdigest()})
if not LIST_ONLY:
    os.makedirs(OUT, exist_ok=True)
    json.dump(manifest, open(os.path.join(OUT, 'manifest.json'), 'w', encoding='utf-8'), indent=1)
    tot = sum(x.get('size', 0) for x in manifest)
    print(f'da chon {sum(1 for x in manifest if x["type"]=="file")} file + {sum(1 for x in manifest if x["type"]=="symlink")} symlink, tong {tot/1e6:.2f} MB')
