#!/usr/bin/env python3
"""glremote — máy chủ 64-bit: đọc lệnh GLES2 do shim 32-bit (libglremote.so) ghi vào /tmp/glremote.shm
rồi thực thi trên GPU PowerVR thật qua SDL2 (ctypes). Giao thức khớp armhf_gl/client/glremote/src/lib.rs.
Chạy khi MainUI đã thoát. Môi trường: LD_LIBRARY_PATH=/usr/trimui/lib:/mnt/SDCARD/System/lib PYTHONHOME=/mnt/SDCARD/System
Tham số: gl_server.py [--log FILE] [--seconds N]  (N: tự thoát sau N giây không hoạt động; mặc định 20)"""
import ctypes, mmap, os, sys, time, struct

LOGF = None
def log(*a):
    s = ' '.join(str(x) for x in a)
    if LOGF: LOGF.write(s + '\n'); LOGF.flush()
    print(s, flush=True)

args = sys.argv[1:]
idle_limit = 20.0
i = 0
while i < len(args):
    if args[i] == '--log': LOGF = open(args[i + 1], 'w'); i += 2
    elif args[i] == '--seconds': idle_limit = float(args[i + 1]); i += 2
    else: i += 1

# ---- bố cục bộ nhớ chung ----
RING_WORDS = 1 << 20; HDR_WORDS = 1024; RESP_WORDS = 16384
SHM_BYTES = (HDR_WORDS + RING_WORDS + RESP_WORDS) * 4
H_MAGIC, H_READY, H_RPOS, H_WPOS, H_RESP_SEQ, H_WIDTH, H_HEIGHT, H_STR = 0, 2, 4, 5, 6, 8, 9, 64
MAGIC = 0x4D524C47
PATH = '/tmp/glremote.shm'
with open(PATH, 'wb') as f: f.truncate(SHM_BYTES)
fd = os.open(PATH, os.O_RDWR)
mm = mmap.mmap(fd, SHM_BYTES)
mv = memoryview(mm)
U = mv.cast('I'); S = mv.cast('i'); F = mv.cast('f')
BASE = ctypes.addressof(ctypes.c_char.from_buffer(mm))
RING0 = HDR_WORDS; RESP0 = HDR_WORDS + RING_WORDS
MASK = RING_WORDS - 1

# ---- SDL + GLES ----
sdl = ctypes.CDLL('libSDL2-2.0.so.0')
sdl.SDL_GetError.restype = ctypes.c_char_p
def sdlerr(): return sdl.SDL_GetError().decode()
if sdl.SDL_Init(0x20) != 0: log('SDL_Init loi', sdlerr()); sys.exit(1)
for a, v in ((17, 2), (18, 0), (21, 4)): sdl.SDL_GL_SetAttribute(a, v)
sdl.SDL_GL_SetAttribute(5, 8); sdl.SDL_GL_SetAttribute(13, 0)  # (không bắt buộc; bỏ qua nếu lỗi)
sdl.SDL_CreateWindow.restype = ctypes.c_void_p
sdl.SDL_CreateWindow.argtypes = [ctypes.c_char_p, ctypes.c_int, ctypes.c_int, ctypes.c_int, ctypes.c_int, ctypes.c_uint32]
win = sdl.SDL_CreateWindow(b'glremote', 0, 0, 1024, 768, 0x2 | 0x1 | 0x4)
if not win: log('CreateWindow loi', sdlerr()); sys.exit(1)
sdl.SDL_GL_CreateContext.restype = ctypes.c_void_p; sdl.SDL_GL_CreateContext.argtypes = [ctypes.c_void_p]
ctx = sdl.SDL_GL_CreateContext(win)
if not ctx: log('CreateContext loi', sdlerr()); sys.exit(1)
sdl.SDL_GL_SwapWindow.argtypes = [ctypes.c_void_p]
sdl.SDL_GL_SetSwapInterval.argtypes = [ctypes.c_int]; sdl.SDL_GL_SetSwapInterval(1)
sdl.SDL_GL_GetProcAddress.restype = ctypes.c_void_p; sdl.SDL_GL_GetProcAddress.argtypes = [ctypes.c_char_p]
sdl.SDL_PumpEvents.argtypes = []

u, i32, f32, vp, cp, ub = ctypes.c_uint, ctypes.c_int, ctypes.c_float, ctypes.c_void_p, ctypes.c_char_p, ctypes.c_ubyte
sz = ctypes.c_ssize_t
PI = ctypes.POINTER(i32)
SPEC = {
 'glGetString': (cp, u), 'glClearColor': (None, f32, f32, f32, f32), 'glClear': (None, u), 'glClearDepthf': (None, f32),
 'glClearStencil': (None, i32), 'glViewport': (None, i32, i32, i32, i32), 'glScissor': (None, i32, i32, i32, i32),
 'glEnable': (None, u), 'glDisable': (None, u), 'glBlendFunc': (None, u, u), 'glBlendFuncSeparate': (None, u, u, u, u),
 'glBlendEquationSeparate': (None, u, u), 'glBlendColor': (None, f32, f32, f32, f32), 'glDepthFunc': (None, u),
 'glDepthMask': (None, ub), 'glDepthRangef': (None, f32, f32), 'glColorMask': (None, ub, ub, ub, ub), 'glFrontFace': (None, u),
 'glCullFace': (None, u), 'glLineWidth': (None, f32), 'glHint': (None, u, u), 'glStencilFunc': (None, u, i32, u),
 'glStencilOp': (None, u, u, u), 'glStencilMask': (None, u), 'glPixelStorei': (None, u, i32), 'glFinish': (None,),
 'glGetError': (u,), 'glActiveTexture': (None, u), 'glGetIntegerv': (None, u, PI),
 'glCreateShader': (u, u), 'glShaderSource': (None, u, i32, ctypes.POINTER(cp), PI), 'glCompileShader': (None, u),
 'glGetShaderiv': (None, u, u, PI), 'glGetShaderInfoLog': (None, u, i32, PI, vp), 'glGetProgramInfoLog': (None, u, i32, PI, vp),
 'glCreateProgram': (u,), 'glAttachShader': (None, u, u), 'glLinkProgram': (None, u), 'glGetProgramiv': (None, u, u, PI),
 'glUseProgram': (None, u), 'glBindAttribLocation': (None, u, u, cp), 'glGetAttribLocation': (i32, u, cp),
 'glGetUniformLocation': (i32, u, cp), 'glDeleteShader': (None, u), 'glDeleteProgram': (None, u),
 'glUniform1f': (None, i32, f32), 'glUniform2f': (None, i32, f32, f32), 'glUniform3f': (None, i32, f32, f32, f32),
 'glUniform4f': (None, i32, f32, f32, f32, f32), 'glUniform1i': (None, i32, i32), 'glUniform2i': (None, i32, i32, i32),
 'glUniform3i': (None, i32, i32, i32, i32), 'glUniform4i': (None, i32, i32, i32, i32, i32),
 'glUniform1fv': (None, i32, i32, vp), 'glUniform2fv': (None, i32, i32, vp), 'glUniform3fv': (None, i32, i32, vp), 'glUniform4fv': (None, i32, i32, vp),
 'glUniformMatrix2fv': (None, i32, i32, ub, vp), 'glUniformMatrix3fv': (None, i32, i32, ub, vp), 'glUniformMatrix4fv': (None, i32, i32, ub, vp),
 'glEnableVertexAttribArray': (None, u), 'glDisableVertexAttribArray': (None, u),
 'glVertexAttribPointer': (None, u, i32, u, ub, i32, vp), 'glVertexAttrib4f': (None, u, f32, f32, f32, f32),
 'glGenBuffers': (None, i32, vp), 'glBindBuffer': (None, u, u), 'glBufferData': (None, u, sz, vp, u), 'glBufferSubData': (None, u, sz, sz, vp),
 'glDeleteBuffers': (None, i32, vp), 'glDrawArrays': (None, u, i32, i32), 'glDrawElements': (None, u, i32, u, vp),
 'glGenTextures': (None, i32, vp), 'glBindTexture': (None, u, u), 'glDeleteTextures': (None, i32, vp),
 'glTexParameteri': (None, u, u, i32), 'glGenerateMipmap': (None, u),
 'glTexImage2D': (None, u, i32, i32, i32, i32, i32, u, u, vp), 'glTexSubImage2D': (None, u, i32, i32, i32, i32, i32, u, u, vp),
 'glCopyTexImage2D': (None, u, i32, u, i32, i32, i32, i32, i32),
 'glGenFramebuffers': (None, i32, vp), 'glBindFramebuffer': (None, u, u), 'glFramebufferTexture2D': (None, u, u, u, u, i32),
 'glGenRenderbuffers': (None, i32, vp), 'glBindRenderbuffer': (None, u, u), 'glRenderbufferStorage': (None, u, u, i32, i32),
 'glFramebufferRenderbuffer': (None, u, u, u, u), 'glCheckFramebufferStatus': (u, u),
 'glDeleteFramebuffers': (None, i32, vp), 'glDeleteRenderbuffers': (None, i32, vp),
 'glReadPixels': (None, i32, i32, i32, i32, u, u, vp),
}
GL = {}
for name, spec in SPEC.items():
    p = sdl.SDL_GL_GetProcAddress(name.encode())
    if not p: log('THIEU', name); continue
    GL[name] = ctypes.CFUNCTYPE(spec[0], *spec[1:])(p)
g = type('G', (), GL)  # g.glClear(...)
renderer = g.glGetString(0x1F01) or b'?'
log('GL_RENDERER:', renderer.decode(), '| VERSION:', (g.glGetString(0x1F02) or b'').decode())
rb = renderer[:250] + b'\0'
mm[H_STR * 4:H_STR * 4 + len(rb)] = rb
U[H_WIDTH] = 1024; U[H_HEIGHT] = 768
U[H_RPOS] = 0; U[H_WPOS] = 0; U[H_RESP_SEQ] = 0
U[H_MAGIC] = MAGIC; U[H_READY] = 1

# ---- trạng thái ----
bound_array = [0]; bound_elem = [0]
client_attr = {}                # i -> ctypes buffer (giữ dữ liệu mảng đỉnh phía client)
resp_seq = [0]
frames = [0]; cmds = [0]
errs = {}

def respond(*vals):
    for k, v in enumerate(vals): U[RESP0 + k] = v & 0xFFFFFFFF
    resp_seq[0] = (resp_seq[0] + 1) & 0xFFFFFFFF
    U[H_RESP_SEQ] = resp_seq[0]

def respond_bytes(b):
    n = len(b); mm[RESP0 * 4 + 4:RESP0 * 4 + 4 + n] = b; U[RESP0] = n
    resp_seq[0] = (resp_seq[0] + 1) & 0xFFFFFFFF; U[H_RESP_SEQ] = resp_seq[0]

def ptr(word_index): return BASE + word_index * 4
def blob_bytes(p, nargs, length): s = (p + nargs) * 4; return bytes(mm[s:s + length])
def gen(fn, n):
    arr = (u * n)(); fn(n, ctypes.addressof(arr)); respond(*arr)
def glerr(tag):
    e = g.glGetError()
    if e: errs[tag] = errs.get(tag, 0) + 1; (errs[tag] <= 3) and log('GL error 0x%x tras %s' % (e, tag))

# ---- thực thi (p = chỉ số từ đầu tiên của tham số trong U/S/F) ----
def op_hello(p, n): respond(1024, 768)
def op_swap(p, n):
    sdl.SDL_GL_SwapWindow(win); frames[0] += 1
    if frames[0] % 8 == 0: sdl.SDL_PumpEvents()
    respond(0)
def op_clear_color(p, n): g.glClearColor(F[p], F[p + 1], F[p + 2], F[p + 3])
def op_clear(p, n): g.glClear(U[p])
def op_viewport(p, n): g.glViewport(S[p], S[p + 1], S[p + 2], S[p + 3])
def op_create_shader(p, n): respond(g.glCreateShader(U[p]))
def op_shader_source(p, n):
    sh, ln = U[p], U[p + 1]; src = ctypes.create_string_buffer(blob_bytes(p, 2, ln), ln + 1)
    arr = (cp * 1)(ctypes.cast(src, cp)); l = (i32 * 1)(ln)
    g.glShaderSource(sh, 1, arr, l)
def op_compile_shader(p, n):
    g.glCompileShader(U[p])
    ok = i32(); g.glGetShaderiv(U[p], 0x8B81, ctypes.byref(ok))
    if not ok.value:
        buf = ctypes.create_string_buffer(2048); g.glGetShaderInfoLog(U[p], 2048, None, buf); log('SHADER LOI:', buf.value.decode(errors='replace'))
def op_get_shader_iv(p, n): v = i32(); g.glGetShaderiv(U[p], U[p + 1], ctypes.byref(v)); respond(v.value)
def op_get_shader_log(p, n):
    buf = ctypes.create_string_buffer(4096); l = i32()
    (g.glGetProgramInfoLog if U[p + 1] else g.glGetShaderInfoLog)(U[p], 4096, ctypes.byref(l), ctypes.addressof(buf))
    respond_bytes(buf.raw[:l.value])
def op_create_program(p, n): respond(g.glCreateProgram())
def op_attach_shader(p, n): g.glAttachShader(U[p], U[p + 1])
def op_link_program(p, n):
    g.glLinkProgram(U[p]); v = i32(); g.glGetProgramiv(U[p], 0x8B82, ctypes.byref(v))
    if not v.value:
        buf = ctypes.create_string_buffer(2048); g.glGetProgramInfoLog(U[p], 2048, None, ctypes.addressof(buf)); log('LINK LOI:', buf.value.decode(errors='replace'))
def op_get_program_iv(p, n): v = i32(); g.glGetProgramiv(U[p], U[p + 1], ctypes.byref(v)); respond(v.value)
def op_use_program(p, n): g.glUseProgram(U[p])
def op_get_attrib_loc(p, n): respond(g.glGetAttribLocation(U[p], blob_bytes(p, 2, U[p + 1])))
def op_get_uniform_loc(p, n): respond(g.glGetUniformLocation(U[p], blob_bytes(p, 2, U[p + 1])))
def op_bind_attrib_loc(p, n): g.glBindAttribLocation(U[p], U[p + 1], blob_bytes(p, 3, U[p + 2]))
def op_uniform_f(p, n):
    loc, c = S[p], U[p + 1]; v = [F[p + 2 + k] for k in range(c)]
    (None, g.glUniform1f, g.glUniform2f, g.glUniform3f, g.glUniform4f)[c](loc, *v)
def op_uniform_i(p, n):
    loc, c = S[p], U[p + 1]; v = [S[p + 2 + k] for k in range(c)]
    (None, g.glUniform1i, g.glUniform2i, g.glUniform3i, g.glUniform4i)[c](loc, *v)
def op_uniform_fv(p, n): (None, g.glUniform1fv, g.glUniform2fv, g.glUniform3fv, g.glUniform4fv)[U[p + 1]](S[p], U[p + 2], ptr(p + 3))
def op_uniform_mat(p, n): (None, None, g.glUniformMatrix2fv, g.glUniformMatrix3fv, g.glUniformMatrix4fv)[U[p + 1]](S[p], U[p + 2], U[p + 3], ptr(p + 4))
def op_enable_vaa(p, n): g.glEnableVertexAttribArray(U[p])
def op_disable_vaa(p, n): g.glDisableVertexAttribArray(U[p])
def op_vap_buffer(p, n): g.glVertexAttribPointer(U[p], S[p + 1], U[p + 2], U[p + 3], S[p + 4], U[p + 5])
def op_vap_const(p, n): g.glVertexAttrib4f(U[p], F[p + 1], F[p + 2], F[p + 3], F[p + 4])
def op_gen_buffers(p, n): gen(g.glGenBuffers, U[p])
def op_bind_buffer(p, n):
    if U[p] == 0x8892: bound_array[0] = U[p + 1]
    elif U[p] == 0x8893: bound_elem[0] = U[p + 1]
    g.glBindBuffer(U[p], U[p + 1])
def op_buffer_data(p, n): g.glBufferData(U[p], U[p + 1], ptr(p + 4) if U[p + 3] else None, U[p + 2])
def op_buffer_subdata(p, n): g.glBufferSubData(U[p], U[p + 1], U[p + 2], ptr(p + 3))
def op_upload_attrib(p, n):
    idx, size, ty, norm, stride, nbytes = U[p], U[p + 1], U[p + 2], U[p + 3], U[p + 4], U[p + 5]
    buf = ctypes.create_string_buffer(blob_bytes(p, 6, nbytes), max(nbytes, 1)); client_attr[idx] = buf
    if bound_array[0]: g.glBindBuffer(0x8892, 0)
    g.glVertexAttribPointer(idx, size, ty, norm, stride, ctypes.addressof(buf))
    if bound_array[0]: g.glBindBuffer(0x8892, bound_array[0])
def op_draw_arrays(p, n): g.glDrawArrays(U[p], S[p + 1], S[p + 2])
def op_draw_elements(p, n):
    mode, count, ty, off, client = U[p], S[p + 1], U[p + 2], U[p + 3], U[p + 4]
    if client:
        buf = ctypes.create_string_buffer(blob_bytes(p, 5, count * {0x1401: 1, 0x1403: 2, 0x1405: 4}[ty]))
        if bound_elem[0]: g.glBindBuffer(0x8893, 0)
        g.glDrawElements(mode, count, ty, ctypes.addressof(buf))
        if bound_elem[0]: g.glBindBuffer(0x8893, bound_elem[0])
    else:
        g.glDrawElements(mode, count, ty, off)
def op_gen_textures(p, n): gen(g.glGenTextures, U[p])
def op_bind_texture(p, n): g.glBindTexture(U[p], U[p + 1])
def op_tex_image(p, n):
    has = (n - 2) > 8
    g.glTexImage2D(U[p], S[p + 1], S[p + 2], S[p + 3], S[p + 4], S[p + 5], U[p + 6], U[p + 7], ptr(p + 8) if has else None)
def op_tex_subimage(p, n): g.glTexSubImage2D(U[p], S[p + 1], S[p + 2], S[p + 3], S[p + 4], S[p + 5], U[p + 6], U[p + 7], ptr(p + 8))
def op_tex_param_i(p, n): g.glTexParameteri(U[p], U[p + 1], S[p + 2])
def op_active_texture(p, n): g.glActiveTexture(U[p])
def op_enable(p, n): g.glEnable(U[p])
def op_disable(p, n): g.glDisable(U[p])
def op_blend_func(p, n): g.glBlendFunc(U[p], U[p + 1])
def op_blend_func_sep(p, n): g.glBlendFuncSeparate(U[p], U[p + 1], U[p + 2], U[p + 3])
def op_depth_func(p, n): g.glDepthFunc(U[p])
def op_depth_mask(p, n): g.glDepthMask(U[p])
def op_scissor(p, n): g.glScissor(S[p], S[p + 1], S[p + 2], S[p + 3])
def op_pixel_store(p, n): g.glPixelStorei(U[p], S[p + 1])
def op_finish(p, n): g.glFinish(); respond(0)
def op_front_face(p, n): g.glFrontFace(U[p])
def op_cull_face(p, n): g.glCullFace(U[p])
def op_delete_objs(p, n):
    kind, cnt = U[p], U[p + 1]; a = (u * cnt)(*[U[p + 2 + k] for k in range(cnt)]); ad = ctypes.addressof(a)
    if kind == 0: g.glDeleteShader(a[0])
    elif kind == 1: g.glDeleteProgram(a[0])
    elif kind == 2: g.glDeleteBuffers(cnt, ad)
    elif kind == 3: g.glDeleteTextures(cnt, ad)
    elif kind == 4: g.glDeleteFramebuffers(cnt, ad)
    elif kind == 5: g.glDeleteRenderbuffers(cnt, ad)
def op_blend_eq(p, n): g.glBlendEquationSeparate(U[p], U[p + 1])
def op_color_mask(p, n): g.glColorMask(U[p], U[p + 1], U[p + 2], U[p + 3])
def op_gen_mipmap(p, n): g.glGenerateMipmap(U[p])
def op_get_integerv(p, n):
    arr = (i32 * 16)(); g.glGetIntegerv(U[p], ctypes.cast(arr, PI))
    cnt = {0x0BA2: 4, 0x0C10: 4, 0x0D3A: 2, 0x846E: 2, 0x846D: 2}.get(U[p], 1)
    respond(cnt, *arr[:cnt])
def op_bye(p, n): pass
def op_blend_color(p, n): g.glBlendColor(F[p], F[p + 1], F[p + 2], F[p + 3])
def op_stencil_func(p, n): g.glStencilFunc(U[p], S[p + 1], U[p + 2])
def op_stencil_op(p, n): g.glStencilOp(U[p], U[p + 1], U[p + 2])
def op_stencil_mask(p, n): g.glStencilMask(U[p])
def op_clear_depth(p, n): g.glClearDepthf(F[p])
def op_clear_stencil(p, n): g.glClearStencil(S[p])
def op_line_width(p, n): g.glLineWidth(F[p])
def op_hint(p, n): g.glHint(U[p], U[p + 1])
def op_depth_range(p, n): g.glDepthRangef(F[p], F[p + 1])
def op_gen_fbo(p, n): gen(g.glGenFramebuffers, U[p])
def op_bind_fbo(p, n): g.glBindFramebuffer(U[p], U[p + 1])
def op_fb_tex2d(p, n): g.glFramebufferTexture2D(U[p], U[p + 1], U[p + 2], U[p + 3], S[p + 4])
def op_gen_rb(p, n): gen(g.glGenRenderbuffers, U[p])
def op_bind_rb(p, n): g.glBindRenderbuffer(U[p], U[p + 1])
def op_rb_storage(p, n): g.glRenderbufferStorage(U[p], U[p + 1], S[p + 2], S[p + 3])
def op_fb_rb(p, n): g.glFramebufferRenderbuffer(U[p], U[p + 1], U[p + 2], U[p + 3])
def op_check_fb(p, n): respond(g.glCheckFramebufferStatus(U[p]))
def op_read_pixels(p, n):
    row = U[p + 6]; h = S[p + 3]; total = row * h
    buf = ctypes.create_string_buffer(total)
    g.glReadPixels(S[p], S[p + 1], S[p + 2], h, U[p + 4], U[p + 5], ctypes.addressof(buf))
    mm[RESP0 * 4:RESP0 * 4 + total] = buf.raw; resp_seq[0] = (resp_seq[0] + 1) & 0xFFFFFFFF; U[H_RESP_SEQ] = resp_seq[0]
def op_copy_tex_image(p, n): g.glCopyTexImage2D(U[p], S[p + 1], U[p + 2], S[p + 3], S[p + 4], S[p + 5], S[p + 6], S[p + 7])

OPS = {1: op_hello, 2: op_swap, 3: op_clear_color, 4: op_clear, 5: op_viewport, 6: op_create_shader, 7: op_shader_source,
 8: op_compile_shader, 9: op_get_shader_iv, 10: op_get_shader_log, 11: op_create_program, 12: op_attach_shader, 13: op_link_program,
 14: op_get_program_iv, 15: op_use_program, 16: op_get_attrib_loc, 17: op_get_uniform_loc, 18: op_uniform_f, 19: op_uniform_i,
 20: op_uniform_mat, 21: op_enable_vaa, 22: op_disable_vaa, 23: op_vap_buffer, 25: op_gen_buffers, 26: op_bind_buffer,
 27: op_buffer_data, 28: op_buffer_subdata, 29: op_draw_arrays, 30: op_draw_elements, 31: op_gen_textures, 32: op_bind_texture,
 33: op_tex_image, 34: op_tex_subimage, 35: op_tex_param_i, 36: op_active_texture, 37: op_enable, 38: op_disable,
 39: op_blend_func, 40: op_blend_func_sep, 41: op_depth_func, 42: op_depth_mask, 43: op_scissor, 44: op_pixel_store,
 45: op_finish, 46: op_front_face, 47: op_cull_face, 48: op_delete_objs, 49: op_blend_eq, 50: op_color_mask, 51: op_gen_mipmap,
 52: op_get_integerv, 53: op_upload_attrib, 54: op_bye, 55: op_uniform_fv, 56: op_blend_color, 57: op_stencil_func,
 58: op_stencil_op, 59: op_stencil_mask, 60: op_clear_depth, 61: op_clear_stencil, 62: op_line_width, 63: op_hint,
 64: op_depth_range, 65: op_gen_fbo, 66: op_bind_fbo, 67: op_fb_tex2d, 68: op_gen_rb, 69: op_bind_rb, 70: op_rb_storage,
 71: op_fb_rb, 72: op_check_fb, 73: op_read_pixels, 74: op_vap_const, 75: op_bind_attrib_loc, 77: op_copy_tex_image}

log('san sang, cho client (%s) ...' % PATH)
R = 0; t_last = time.time(); t_start = None; bye = False; unknown = set()
try:
    while not bye:
        w = U[H_WPOS]
        if R == w:
            if time.time() - t_last > idle_limit: log('het thoi gian cho'); break
            time.sleep(0.0005); continue
        t_last = time.time()
        if t_start is None: t_start = t_last
        k = 0
        while R != w:
            idx = RING0 + (R & MASK); op = U[idx]; n = U[idx + 1]
            if op == 0xFFFF: R = (R + n) & 0xFFFFFFFF; continue
            fn = OPS.get(op)
            if fn is None:
                if op not in unknown: unknown.add(op); log('lenh la', op)
            else:
                fn(idx + 2, n)
                if op == 54: bye = True
            cmds[0] += 1
            R = (R + n) & 0xFFFFFFFF; k += 1
            if (k & 63) == 0: U[H_RPOS] = R
            if bye: break
        U[H_RPOS] = R
finally:
    dt = time.time() - (t_start or time.time())
    log('xong: %d khung, %d lenh trong %.1fs => %.1f FPS, %.0f lenh/s' % (frames[0], cmds[0], dt, frames[0] / dt if dt else 0, cmds[0] / dt if dt else 0))
    if errs: log('loi GL:', errs)
    sdl.SDL_GL_DeleteContext.argtypes = [ctypes.c_void_p]; sdl.SDL_GL_DeleteContext(ctx)
    sdl.SDL_DestroyWindow.argtypes = [ctypes.c_void_p]; sdl.SDL_DestroyWindow(win); sdl.SDL_Quit()
    try: os.unlink(PATH)
    except OSError: pass
