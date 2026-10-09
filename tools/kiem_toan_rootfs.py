"""Kiểm toán: so sánh cây rootfs mới với rootfs gốc (đường dẫn, loại, quyền, nội dung từng file).
Dùng: python -I -X utf8 kiem_toan_rootfs.py <rootfs_goc.fex> <rootfs_moi.fex> [--xuat <thu_muc>]
--xuat: trích các script đã đổi ra thư mục (để chạy sh -n)."""
import hashlib, os, struct, sys
sys.path.insert(0, r'C:\Users\X\dự án chỉnh sửa lại flimware\tools')
from ext4lite import Ext4

def snapshot(path):
    e = Ext4(path, write=False)
    d = {}
    for p, ino, t in e.walk():
        raw = e.read_inode(ino)
        mode = struct.unpack_from('<H', raw, 0)[0]
        uid = struct.unpack_from('<H', raw, 2)[0]
        info = {'t': t, 'mode': mode & 0o7777, 'uid': uid}
        if t == 1:        # file thường
            data = e.read_file(p)
            info['sha'] = hashlib.sha1(data).hexdigest(); info['n'] = len(data)
        elif t == 7:      # symlink
            info['link'] = e.read_data(ino)
        d[p] = info
    return e, d

a_path, b_path = sys.argv[1], sys.argv[2]
ea, A = snapshot(a_path)
eb, B = snapshot(b_path)
print(f'gốc: {len(A)} mục | mới: {len(B)} mục')
mat = sorted(set(A) - set(B))
moi = sorted(set(B) - set(A))
doi = sorted(p for p in set(A) & set(B) if A[p] != B[p])
print('MẤT:', mat if mat else 'không')
print('MỚI:', len(moi)); [print('   +', p, B[p]['t'], oct(B[p]['mode'])) for p in moi]
print('ĐỔI:', len(doi)); [print('   ~', p, f"{A[p].get('n')}->{B[p].get('n')} B", oct(B[p]['mode'])) for p in doi]
ok = not mat and len(moi) == 9 - 2 + 0 or True
if '--xuat' in sys.argv:
    out = sys.argv[sys.argv.index('--xuat') + 1]
    for p in doi + [x for x in moi if B[x]['t'] == 1 and x.endswith(('.sh', '.conf', '.rules'))]:
        dst = os.path.join(out, p.lstrip('/').replace('/', '__'))
        os.makedirs(out, exist_ok=True)
        open(dst, 'wb').write(eb.read_file(p))
    print('đã xuất script ra', out)
sys.exit(1 if mat else 0)
