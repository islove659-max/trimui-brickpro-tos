"""Tao manifest.json cho runtime glremote (dung boi tools/build_fw_v2.py --glremote)."""
import hashlib, json, os, sys
stage, dist = sys.argv[1], sys.argv[2]
R = '/usr/lib/glremote'
files = [  # (nguon tuong doi stage, dich trong rootfs, mode)
    ('glserver', R + '/glserver', 0o755),
    ('lib32/libglremote.so', R + '/lib32/libglremote.so', 0o755),
    ('lib32/libasound.so.2', R + '/lib32/libasound.so.2', 0o755),
    ('lib32/libxkbcommon.so.0', R + '/lib32/libxkbcommon.so.0', 0o755),
    ('lib32/libpulse.so.0', R + '/lib32/libpulse.so.0', 0o755),
    ('lib32/libsdl_deps_stub.so', R + '/lib32/libsdl_deps_stub.so', 0o755),
    ('lib32/libdrm.so.2', R + '/lib32/libdrm.so.2', 0o755),
    ('lib32-sdl2/libSDL2-2.0.so.0', R + '/lib32-sdl2/libSDL2-2.0.so.0', 0o755),
    ('glremote_run', '/usr/bin/glremote_run', 0o755),
    ('README.txt', R + '/README.txt', 0o644),
]
files.append(('licenses/libudev-copyright.txt', R + '/licenses/libudev-copyright.txt', 0o644))
files.append(('trimui-controllerdb.txt', R + '/trimui-controllerdb.txt', 0o644))
udev = os.path.join(stage, 'lib32', 'libudev.so.1')
if os.path.isfile(udev): files.append(('lib32/libudev.so.1', R + '/lib32/libudev.so.1', 0o755))
gmlibs = os.path.join(stage, 'gmlibs')
if os.path.isdir(gmlibs):
    files += [('gmlibs/' + n, R + '/gmlibs/' + n, 0o755) for n in sorted(os.listdir(gmlibs)) if os.path.isfile(os.path.join(gmlibs,n))]
links = {  # ten -> muc tieu (tuong doi, cung thu muc)
    'libEGL.so.1': 'libglremote.so', 'libEGL.so': 'libglremote.so', 'libGLESv2.so.2': 'libglremote.so', 'libGLESv2.so': 'libglremote.so',
    'libGLESv1_CM.so.1': 'libglremote.so', 'libGL.so.1': 'libglremote.so', 'libGL.so': 'libglremote.so',
    'libOpenGL.so.0': 'libglremote.so', 'libGLX.so.0': 'libglremote.so', 'libGLdispatch.so.0': 'libglremote.so',
    'libgbm.so.1': 'libdrm.so.2',
}
for n in ('libX11.so.6', 'libXext.so.6', 'libXcursor.so.1', 'libXinerama.so.1', 'libXi.so.6', 'libXrandr.so.2', 'libXss.so.1',
          'libXxf86vm.so.1', 'libwayland-egl.so.1', 'libwayland-client.so.0', 'libwayland-cursor.so.0'):
    links[n] = 'libsdl_deps_stub.so'
man = [{'type': 'dir', 'path': '/usr/lib/glremote'}, {'type': 'dir', 'path': R + '/lib32'}, {'type': 'dir', 'path': R + '/lib32-sdl2'}]
man.append({'type':'dir','path':R+'/gmlibs'})
man.append({'type':'dir','path':R+'/licenses'})
out_files = os.path.join(dist, 'files')
for src, dst, mode in files:
    data = open(os.path.join(stage, src.replace('/', os.sep)), 'rb').read()
    if src in ('glremote_run', 'README.txt'): data = data.replace(b'\r\n', b'\n')
    p = os.path.join(out_files, dst.lstrip('/').replace('/', os.sep)); os.makedirs(os.path.dirname(p), exist_ok=True); open(p, 'wb').write(data)
    man.append({'type': 'file', 'path': dst, 'mode': mode, 'sha256': hashlib.sha256(data).hexdigest(), 'size': len(data)})
for name, tgt in sorted(links.items()):
    man.append({'type': 'symlink', 'path': R + '/lib32/' + name, 'target': tgt})
open(os.path.join(dist, 'manifest.json'), 'w', encoding='utf-8').write(json.dumps(man, indent=1))
print('manifest:', sum(1 for m in man if m['type'] == 'file'), 'file,', len(links), 'symlink,', sum(1 for m in man if m['type'] == 'dir'), 'thu muc;', sum(m.get('size', 0) for m in man) // 1024, 'KB')
