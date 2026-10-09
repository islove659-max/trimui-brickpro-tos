#!/usr/bin/env python3
"""Chặng 1 (hướng B): thử tạo ngữ cảnh GLES2 THẬT trên PowerVR qua SDL2 hệ thống (64-bit) bằng ctypes và vẽ.
Chạy trên máy (MainUI phải thoát, ví dụ qua /tmp/cmd_to_run.sh). Ghi kết quả vào /mnt/UDISK/gl_poc.log.
Môi trường: LD_LIBRARY_PATH=/usr/trimui/lib:/mnt/SDCARD/System/lib  PYTHONHOME=/mnt/SDCARD/System
Đo: số khung/giây khi (a) chỉ clear + swap, (b) tam giác có shader, (c) 1000 lệnh GL nhẹ mỗi khung (mô phỏng game 2D)."""
import ctypes, os, sys, time

LOG = open('/mnt/UDISK/gl_poc.log', 'w', buffering=1)
def log(*a):
    s = ' '.join(str(x) for x in a); print(s); LOG.write(s + '\n')

try:
    sdl = ctypes.CDLL('libSDL2-2.0.so.0')
except OSError as e:
    log('KHONG nap duoc libSDL2:', e); sys.exit(1)

sdl.SDL_GetError.restype = ctypes.c_char_p
def err(): return sdl.SDL_GetError().decode()
SDL_INIT_VIDEO = 0x20
if sdl.SDL_Init(SDL_INIT_VIDEO) != 0: log('SDL_Init loi:', err()); sys.exit(1)
log('SDL video driver:', (lambda f: (setattr(f, 'restype', ctypes.c_char_p), f())[1])(sdl.SDL_GetCurrentVideoDriver))
# GLES2
for attr, val in ((17, 2), (18, 0), (21, 4)):   # MAJOR=2, MINOR=0, PROFILE_ES
    sdl.SDL_GL_SetAttribute(attr, val)
sdl.SDL_CreateWindow.restype = ctypes.c_void_p
sdl.SDL_CreateWindow.argtypes = [ctypes.c_char_p, ctypes.c_int, ctypes.c_int, ctypes.c_int, ctypes.c_int, ctypes.c_uint32]
win = sdl.SDL_CreateWindow(b'gl_poc', 0, 0, 1024, 768, 0x2 | 0x1 | 0x4)  # OPENGL | FULLSCREEN | SHOWN
if not win: log('SDL_CreateWindow loi:', err()); sys.exit(1)
sdl.SDL_GL_CreateContext.restype = ctypes.c_void_p
sdl.SDL_GL_CreateContext.argtypes = [ctypes.c_void_p]
ctx = sdl.SDL_GL_CreateContext(win)
if not ctx: log('SDL_GL_CreateContext loi:', err()); sys.exit(1)
sdl.SDL_GL_SwapWindow.argtypes = [ctypes.c_void_p]   # BAT BUOC: neu khong con tro cua so 64-bit bi cat 32-bit => segfault
sdl.SDL_GL_SetSwapInterval.argtypes = [ctypes.c_int]
sdl.SDL_GL_SetSwapInterval(1)
sdl.SDL_GL_GetProcAddress.restype = ctypes.c_void_p
sdl.SDL_GL_GetProcAddress.argtypes = [ctypes.c_char_p]

def fn(name, restype, *argtypes):
    p = sdl.SDL_GL_GetProcAddress(name.encode())
    if not p: log('THIEU ham', name); return None
    return ctypes.CFUNCTYPE(restype, *argtypes)(p)

c_uint, c_int, c_float, c_void_p, c_char_p = ctypes.c_uint, ctypes.c_int, ctypes.c_float, ctypes.c_void_p, ctypes.c_char_p
glGetString = fn('glGetString', c_char_p, c_uint)
glClearColor = fn('glClearColor', None, c_float, c_float, c_float, c_float)
glClear = fn('glClear', None, c_uint)
glViewport = fn('glViewport', None, c_int, c_int, c_int, c_int)
glCreateShader = fn('glCreateShader', c_uint, c_uint)
glShaderSource = fn('glShaderSource', None, c_uint, c_int, ctypes.POINTER(c_char_p), ctypes.POINTER(c_int))
glCompileShader = fn('glCompileShader', None, c_uint)
glGetShaderiv = fn('glGetShaderiv', None, c_uint, c_uint, ctypes.POINTER(c_int))
glCreateProgram = fn('glCreateProgram', c_uint)
glAttachShader = fn('glAttachShader', None, c_uint, c_uint)
glLinkProgram = fn('glLinkProgram', None, c_uint)
glUseProgram = fn('glUseProgram', None, c_uint)
glGetAttribLocation = fn('glGetAttribLocation', c_int, c_uint, c_char_p)
glGetUniformLocation = fn('glGetUniformLocation', c_int, c_uint, c_char_p)
glUniform4f = fn('glUniform4f', None, c_int, c_float, c_float, c_float, c_float)
glEnableVertexAttribArray = fn('glEnableVertexAttribArray', None, c_uint)
glVertexAttribPointer = fn('glVertexAttribPointer', None, c_uint, c_int, c_uint, ctypes.c_ubyte, c_int, c_void_p)
glDrawArrays = fn('glDrawArrays', None, c_uint, c_int, c_int)
glGetError = fn('glGetError', c_uint)

log('GL_VENDOR  :', glGetString(0x1F00).decode())
log('GL_RENDERER:', glGetString(0x1F01).decode())
log('GL_VERSION :', glGetString(0x1F02).decode())
log('GL_EXTENSIONS (200 ky tu dau):', glGetString(0x1F03).decode()[:200])
w, h = ctypes.c_int(), ctypes.c_int()
sdl.SDL_GL_GetDrawableSize.argtypes = [ctypes.c_void_p, ctypes.POINTER(c_int), ctypes.POINTER(c_int)]
sdl.SDL_GL_GetDrawableSize(win, ctypes.byref(w), ctypes.byref(h)); log('drawable:', w.value, 'x', h.value)

def compile_shader(kind, src):
    s = glCreateShader(kind)
    arr = (c_char_p * 1)(src.encode())
    glShaderSource(s, 1, arr, None); glCompileShader(s)
    ok = c_int(); glGetShaderiv(s, 0x8B81, ctypes.byref(ok))
    if not ok.value: log('shader loi'); sys.exit(1)
    return s
vs = compile_shader(0x8B31, 'attribute vec2 p; void main(){ gl_Position = vec4(p, 0.0, 1.0); }')
fs = compile_shader(0x8B30, 'precision mediump float; uniform vec4 c; void main(){ gl_FragColor = c; }')
prog = glCreateProgram(); glAttachShader(prog, vs); glAttachShader(prog, fs); glLinkProgram(prog); glUseProgram(prog)
loc = glGetAttribLocation(prog, b'p'); cu = glGetUniformLocation(prog, b'c')
tri = (c_float * 6)(-0.5, -0.5, 0.5, -0.5, 0.0, 0.6)
glEnableVertexAttribArray(loc)
glVertexAttribPointer(loc, 2, 0x1406, 0, 0, ctypes.cast(tri, c_void_p))

def run(label, seconds, per_frame):
    n = 0; t0 = time.time(); worst = 0.0; tp = t0
    while time.time() - t0 < seconds:
        per_frame(n)
        sdl.SDL_GL_SwapWindow(win)
        n += 1; t = time.time(); worst = max(worst, t - tp); tp = t
    dt = time.time() - t0
    log('%-46s %6.1f FPS  (khung cham nhat %.1f ms, GL error=0x%x)' % (label, n / dt, worst * 1000, glGetError()))

def f_clear(n):
    glClearColor((n % 60) / 60.0, 0.1, 0.3, 1.0); glClear(0x4000)
def f_tri(n):
    glClearColor(0.05, 0.08, 0.2, 1.0); glClear(0x4000)
    glUniform4f(cu, 1.0, (n % 60) / 60.0, 0.2, 1.0); glDrawArrays(4, 0, 3)
def f_many(n):
    glClearColor(0.05, 0.08, 0.2, 1.0); glClear(0x4000)
    for i in range(250):                      # ~1000 lenh GL/khung (4 lenh moi vong)
        glUniform4f(cu, (i % 7) / 7.0, (i % 5) / 5.0, 0.3, 1.0)
        glViewport(0, 0, 1024, 768); glDrawArrays(4, 0, 3)
        glGetError()
log('--- do hieu nang (vsync BAT)')
run('(a) chi clear + swap', 4, f_clear)
run('(b) 1 tam giac co shader', 4, f_tri)
run('(c) ~1000 lenh GL/khung qua ctypes', 5, f_many)
log('XONG')
sdl.SDL_GL_DeleteContext.argtypes = [ctypes.c_void_p]; sdl.SDL_GL_DeleteContext(ctx)
sdl.SDL_DestroyWindow.argtypes = [ctypes.c_void_p]; sdl.SDL_DestroyWindow(win); sdl.SDL_Quit()
