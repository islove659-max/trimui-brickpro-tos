//! glserver — máy chủ glremote bằng Rust (aarch64), không libc/std.
//! Đọc lệnh GLES2 từ shim 32-bit (armhf_gl/client/glremote) qua /tmp/glremote.shm, chạy trên PowerVR qua SDL2 (nạp lúc chạy).
//! Giao thức/bố cục khớp lib.rs của shim và docs/ARMHF_DOHOA.md. Tham số: --seconds N (tự thoát khi rảnh, mặc định 20).
#![no_std]
#![no_main]
#![allow(non_snake_case, non_camel_case_types, static_mut_refs)]

use core::arch::{asm, global_asm};
use core::ffi::c_void;
use core::panic::PanicInfo;
use core::ptr::{null, null_mut, read_volatile, write_volatile};
use core::sync::atomic::{fence, Ordering};

#[panic_handler]
fn panic(_: &PanicInfo) -> ! { out(b"PANIC\n"); exit(2) }
#[no_mangle] pub extern "C" fn rust_eh_personality() {}

global_asm!(".global _start", "_start:", "mov x0, sp", "and sp, x0, #-16", "bl rust_main");

// ---- syscall aarch64 ----
#[inline(always)]
unsafe fn sc(n: usize, a: usize, b: usize, c: usize, d: usize, e: usize, f: usize) -> isize {
    let r: isize;
    asm!("svc 0", in("x8") n, inlateout("x0") a as isize => r, in("x1") b, in("x2") c, in("x3") d, in("x4") e, in("x5") f, options(nostack));
    r
}
fn out(s: &[u8]) { unsafe { sc(64, 1, s.as_ptr() as usize, s.len(), 0, 0, 0); } }
fn out_cstr(p: *const u8) { if p.is_null() { return; } unsafe { let mut n = 0; while read_volatile(p.add(n)) != 0 { n += 1; } out(core::slice::from_raw_parts(p, n)); } }
fn out_num(mut v: u64) { let mut b = [0u8; 20]; let mut i = 20; if v == 0 { i -= 1; b[i] = b'0'; } while v > 0 { i -= 1; b[i] = b'0' + (v % 10) as u8; v /= 10; } out(&b[i..]); }
fn exit(c: usize) -> ! { unsafe { sc(94, c, 0, 0, 0, 0, 0); } loop {} }
fn die(m: &[u8]) -> ! { out(m); exit(1) }
fn sleep_ns(ns: usize) { let ts = [0usize, ns]; unsafe { sc(101, ts.as_ptr() as usize, 0, 0, 0, 0, 0); } }
fn now_ms() -> u64 { let mut ts = [0u64; 2]; unsafe { sc(113, 1, ts.as_mut_ptr() as usize, 0, 0, 0, 0); } ts[0] * 1000 + ts[1] / 1_000_000 }

// ---- hàm bộ nhớ (compiler cần) ----
#[no_mangle] pub unsafe extern "C" fn memcpy(d: *mut u8, s: *const u8, n: usize) -> *mut u8 { let mut i = 0; while i < n { write_volatile(d.add(i), read_volatile(s.add(i))); i += 1; } d }
#[no_mangle] pub unsafe extern "C" fn memmove(d: *mut u8, s: *const u8, n: usize) -> *mut u8 { if (d as usize) < (s as usize) { memcpy(d, s, n) } else { let mut i = n; while i > 0 { i -= 1; write_volatile(d.add(i), read_volatile(s.add(i))); } d } }
#[no_mangle] pub unsafe extern "C" fn memset(d: *mut u8, c: i32, n: usize) -> *mut u8 { let mut i = 0; while i < n { write_volatile(d.add(i), c as u8); i += 1; } d }
#[no_mangle] pub unsafe extern "C" fn memcmp(a: *const u8, b: *const u8, n: usize) -> i32 { let mut i = 0; while i < n { let (x, y) = (read_volatile(a.add(i)), read_volatile(b.add(i))); if x != y { return x as i32 - y as i32; } i += 1; } 0 }
#[no_mangle] pub unsafe extern "C" fn bcmp(a: *const u8, b: *const u8, n: usize) -> i32 { memcmp(a, b, n) }

// ---- SDL2 (khung giả để liên kết; thư viện thật nạp bởi ld.so) ----
#[link(name = "SDL2-2.0")]
extern "C" {
    fn SDL_Init(f: u32) -> i32; fn SDL_GetError() -> *const u8; fn SDL_GL_SetAttribute(a: i32, v: i32) -> i32;
    fn SDL_CreateWindow(t: *const u8, x: i32, y: i32, w: i32, h: i32, f: u32) -> *mut c_void;
    fn SDL_GL_CreateContext(w: *mut c_void) -> *mut c_void; fn SDL_GL_SwapWindow(w: *mut c_void);
    fn SDL_GL_SetSwapInterval(i: i32) -> i32; fn SDL_GL_GetProcAddress(n: *const u8) -> *const c_void; fn SDL_PumpEvents();
    fn SDL_GL_DeleteContext(c: *mut c_void); fn SDL_DestroyWindow(w: *mut c_void); fn SDL_Quit();
}

// ---- bảng hàm GLES2 ----
macro_rules! gl_table {
    ($( $name:ident ( $($a:ty),* ) $(-> $r:ty)? ),* $(,)?) => {
        #[allow(non_snake_case)] struct Gl { $( $name: unsafe extern "C" fn($($a),*) $(-> $r)? ),* }
        unsafe fn null_gl() -> Gl { Gl { $( $name: core::mem::transmute(null_fn as unsafe extern "C" fn() -> usize) ),* } }
        unsafe fn load_gl() -> Gl { Gl { $( $name: {
            let p = SDL_GL_GetProcAddress(concat!(stringify!($name), "\0").as_ptr());
            if p.is_null() { out(b"THIEU "); out(stringify!($name).as_bytes()); out(b"\n"); exit(1) }
            core::mem::transmute(p) } ),* } }
    };
}
unsafe extern "C" fn null_fn() -> usize { 0 }
static mut NULL_MODE: bool = false;
unsafe fn has_arg(argc: usize, argv: *const *const u8, name: &[u8]) -> bool {
    let mut j = 1; while j < argc { let a = *argv.add(j); let mut k = 0; let mut ok = true;
        while k < name.len() { if read_volatile(a.add(k)) != name[k] { ok = false; break; } k += 1; }
        if ok && read_volatile(a.add(name.len())) == 0 { return true; } j += 1; }
    false
}
type PI = *mut i32; type VP = *const c_void; type CP = *const u8;
gl_table! {
    glGetString(u32) -> CP, glClearColor(f32, f32, f32, f32), glClear(u32), glClearDepthf(f32), glClearStencil(i32),
    glViewport(i32, i32, i32, i32), glScissor(i32, i32, i32, i32), glEnable(u32), glDisable(u32), glBlendFunc(u32, u32),
    glBlendFuncSeparate(u32, u32, u32, u32), glBlendEquationSeparate(u32, u32), glBlendColor(f32, f32, f32, f32),
    glDepthFunc(u32), glDepthMask(u8), glDepthRangef(f32, f32), glColorMask(u8, u8, u8, u8), glFrontFace(u32), glCullFace(u32),
    glLineWidth(f32), glHint(u32, u32), glStencilFunc(u32, i32, u32), glStencilOp(u32, u32, u32), glStencilMask(u32),
    glPixelStorei(u32, i32), glFinish(), glGetError() -> u32, glActiveTexture(u32), glGetIntegerv(u32, PI),
    glCreateShader(u32) -> u32, glShaderSource(u32, i32, *const CP, *const i32), glCompileShader(u32), glGetShaderiv(u32, u32, PI),
    glGetShaderInfoLog(u32, i32, PI, *mut u8), glGetProgramInfoLog(u32, i32, PI, *mut u8), glCreateProgram() -> u32,
    glAttachShader(u32, u32), glLinkProgram(u32), glGetProgramiv(u32, u32, PI), glUseProgram(u32),
    glBindAttribLocation(u32, u32, CP), glGetAttribLocation(u32, CP) -> i32, glGetUniformLocation(u32, CP) -> i32,
    glDeleteShader(u32), glDeleteProgram(u32),
    glUniform1f(i32, f32), glUniform2f(i32, f32, f32), glUniform3f(i32, f32, f32, f32), glUniform4f(i32, f32, f32, f32, f32),
    glUniform1i(i32, i32), glUniform2i(i32, i32, i32), glUniform3i(i32, i32, i32, i32), glUniform4i(i32, i32, i32, i32, i32),
    glUniform1fv(i32, i32, VP), glUniform2fv(i32, i32, VP), glUniform3fv(i32, i32, VP), glUniform4fv(i32, i32, VP),
    glUniformMatrix2fv(i32, i32, u8, VP), glUniformMatrix3fv(i32, i32, u8, VP), glUniformMatrix4fv(i32, i32, u8, VP),
    glEnableVertexAttribArray(u32), glDisableVertexAttribArray(u32), glVertexAttribPointer(u32, i32, u32, u8, i32, VP),
    glVertexAttrib4f(u32, f32, f32, f32, f32),
    glGenBuffers(i32, *mut u32), glBindBuffer(u32, u32), glBufferData(u32, isize, VP, u32), glBufferSubData(u32, isize, isize, VP),
    glDeleteBuffers(i32, *const u32), glDrawArrays(u32, i32, i32), glDrawElements(u32, i32, u32, VP),
    glGenTextures(i32, *mut u32), glBindTexture(u32, u32), glDeleteTextures(i32, *const u32), glTexParameteri(u32, u32, i32),
    glGenerateMipmap(u32), glTexImage2D(u32, i32, i32, i32, i32, i32, u32, u32, VP),
    glTexSubImage2D(u32, i32, i32, i32, i32, i32, u32, u32, VP), glCopyTexImage2D(u32, i32, u32, i32, i32, i32, i32, i32),
    glGenFramebuffers(i32, *mut u32), glBindFramebuffer(u32, u32), glFramebufferTexture2D(u32, u32, u32, u32, i32),
    glGetFramebufferAttachmentParameteriv(u32,u32,u32,*mut i32), glGetRenderbufferParameteriv(u32,u32,*mut i32), glGenRenderbuffers(i32, *mut u32), glBindRenderbuffer(u32, u32), glRenderbufferStorage(u32, u32, i32, i32),
    glFramebufferRenderbuffer(u32, u32, u32, u32), glCheckFramebufferStatus(u32) -> u32,
    glDeleteFramebuffers(i32, *const u32), glDeleteRenderbuffers(i32, *const u32),
    glReadPixels(i32, i32, i32, i32, u32, u32, *mut c_void),
    glPolygonOffset(f32, f32), glSampleCoverage(f32, u8), glStencilFuncSeparate(u32, u32, i32, u32), glStencilOpSeparate(u32, u32, u32, u32),
    glStencilMaskSeparate(u32, u32), glCompressedTexImage2D(u32, i32, u32, i32, i32, i32, i32, VP), glCompressedTexSubImage2D(u32, i32, i32, i32, i32, i32, u32, i32, VP),
    glCopyTexSubImage2D(u32, i32, i32, i32, i32, i32, i32, i32),
    glGetActiveUniform(u32, u32, i32, PI, PI, *mut u32, *mut u8), glGetActiveAttrib(u32, u32, i32, PI, PI, *mut u32, *mut u8),
}

// ---- bộ nhớ chung (KHỚP shim) ----
const RING_WORDS: usize = 1 << 20; const HDR_WORDS: usize = 1024; const RESP_WORDS: usize = 16384;
const SHM_BYTES: usize = (HDR_WORDS + RING_WORDS + RESP_WORDS) * 4;
const H_MAGIC: usize = 0; const H_READY: usize = 2; const H_RPOS: usize = 4; const H_WPOS: usize = 5; const H_RESP_SEQ: usize = 6;
const H_WIDTH: usize = 8; const H_HEIGHT: usize = 9; const H_STR: usize = 64;
const MAGIC: u32 = 0x4D524C47;
static mut SHM: *mut u32 = null_mut();
static mut RESP_SEQ: u32 = 0;
#[inline(always)] unsafe fn hdr(i: usize) -> *mut u32 { SHM.add(i) }
#[inline(always)] unsafe fn ring() -> *mut u32 { SHM.add(HDR_WORDS) }
#[inline(always)] unsafe fn resp() -> *mut u32 { SHM.add(HDR_WORDS + RING_WORDS) }
unsafe fn bump_seq() { RESP_SEQ = RESP_SEQ.wrapping_add(1); fence(Ordering::Release); write_volatile(hdr(H_RESP_SEQ), RESP_SEQ); }
unsafe fn respond(v: &[u32]) { let mut i = 0; while i < v.len() { write_volatile(resp().add(i), v[i]); i += 1; } bump_seq(); }

struct St { gl: Gl, win: *mut c_void, bound_array: u32, bound_elem: u32, client_buffers: [u32;16], frames: u64, cmds: u64 }
static mut ST: *mut St = null_mut();

// accessors: p = con trỏ tới từ tham số đầu
#[inline(always)] unsafe fn u(p: *const u32, k: usize) -> u32 { read_volatile(p.add(k)) }
#[inline(always)] unsafe fn s(p: *const u32, k: usize) -> i32 { read_volatile(p.add(k)) as i32 }
#[inline(always)] unsafe fn f(p: *const u32, k: usize) -> f32 { f32::from_bits(read_volatile(p.add(k))) }
unsafe fn name_buf(p: *const u32, nargs: usize, len: usize, buf: &mut [u8; 256]) -> *const u8 {
    let l = if len > 255 { 255 } else { len }; memcpy(buf.as_mut_ptr(), p.add(nargs) as *const u8, l); buf[l] = 0; buf.as_ptr()
}
unsafe fn dispatch(st: &mut St, op: u32, p: *const u32, n: usize) -> bool {
    let g = &st.gl;
    let mut nb = [0u8; 256];
    match op {
        1 => respond(&[1024, 768]),
        2 => { if NULL_MODE { sleep_ns(16_000_000); } else { SDL_GL_SwapWindow(st.win); if st.frames & 7 == 7 { SDL_PumpEvents(); } } st.frames += 1; respond(&[0]); }
        3 => (g.glClearColor)(f(p, 0), f(p, 1), f(p, 2), f(p, 3)),
        4 => (g.glClear)(u(p, 0)),
        5 => (g.glViewport)(s(p, 0), s(p, 1), s(p, 2), s(p, 3)),
        6 => respond(&[(g.glCreateShader)(u(p, 0))]),
        7 => { let ln = u(p, 1) as i32; let src = p.add(2) as CP; (g.glShaderSource)(u(p, 0), 1, &src, &ln); }
        8 => {
            (g.glCompileShader)(u(p, 0)); let mut ok = if NULL_MODE { 1 } else { 0i32 }; (g.glGetShaderiv)(u(p, 0), 0x8B81, &mut ok);
            if ok == 0 { let mut b = [0u8; 1024]; (g.glGetShaderInfoLog)(u(p, 0), 1024, null_mut(), b.as_mut_ptr()); out(b"SHADER LOI: "); out_cstr(b.as_ptr()); out(b"\n"); }
        }
        9 => { let mut v = if NULL_MODE { 1 } else { 0i32 }; (g.glGetShaderiv)(u(p, 0), u(p, 1), &mut v); respond(&[v as u32]); }
        10 => {
            let mut l = 0i32; let d = resp().add(1) as *mut u8;
            if u(p, 1) != 0 { (g.glGetProgramInfoLog)(u(p, 0), 4096, &mut l, d); } else { (g.glGetShaderInfoLog)(u(p, 0), 4096, &mut l, d); }
            write_volatile(resp(), l as u32); bump_seq();
        }
        11 => respond(&[(g.glCreateProgram)()]),
        12 => (g.glAttachShader)(u(p, 0), u(p, 1)),
        13 => {
            (g.glLinkProgram)(u(p, 0)); let mut ok = if NULL_MODE { 1 } else { 0i32 }; (g.glGetProgramiv)(u(p, 0), 0x8B82, &mut ok);
            if ok == 0 { let mut b = [0u8; 1024]; (g.glGetProgramInfoLog)(u(p, 0), 1024, null_mut(), b.as_mut_ptr()); out(b"LINK LOI: "); out_cstr(b.as_ptr()); out(b"\n"); }
        }
        14 => { let mut v = if NULL_MODE { 1 } else { 0i32 }; (g.glGetProgramiv)(u(p, 0), u(p, 1), &mut v); respond(&[v as u32]); }
        15 => (g.glUseProgram)(u(p, 0)),
        16 => { let nm = name_buf(p, 2, u(p, 1) as usize, &mut nb); respond(&[(g.glGetAttribLocation)(u(p, 0), nm) as u32]); }
        17 => { let nm = name_buf(p, 2, u(p, 1) as usize, &mut nb); respond(&[(g.glGetUniformLocation)(u(p, 0), nm) as u32]); }
        75 => { let nm = name_buf(p, 3, u(p, 2) as usize, &mut nb); (g.glBindAttribLocation)(u(p, 0), u(p, 1), nm); }
        18 => { let l = s(p, 0); match u(p, 1) { 1 => (g.glUniform1f)(l, f(p, 2)), 2 => (g.glUniform2f)(l, f(p, 2), f(p, 3)), 3 => (g.glUniform3f)(l, f(p, 2), f(p, 3), f(p, 4)), _ => (g.glUniform4f)(l, f(p, 2), f(p, 3), f(p, 4), f(p, 5)) } }
        19 => { let l = s(p, 0); match u(p, 1) { 1 => (g.glUniform1i)(l, s(p, 2)), 2 => (g.glUniform2i)(l, s(p, 2), s(p, 3)), 3 => (g.glUniform3i)(l, s(p, 2), s(p, 3), s(p, 4)), _ => (g.glUniform4i)(l, s(p, 2), s(p, 3), s(p, 4), s(p, 5)) } }
        55 => { let d = p.add(3) as VP; match u(p, 1) { 1 => (g.glUniform1fv)(s(p, 0), s(p, 2), d), 2 => (g.glUniform2fv)(s(p, 0), s(p, 2), d), 3 => (g.glUniform3fv)(s(p, 0), s(p, 2), d), _ => (g.glUniform4fv)(s(p, 0), s(p, 2), d) } }
        20 => { let d = p.add(4) as VP; let (l, c, t) = (s(p, 0), s(p, 2), u(p, 3) as u8); match u(p, 1) { 2 => (g.glUniformMatrix2fv)(l, c, t, d), 3 => (g.glUniformMatrix3fv)(l, c, t, d), _ => (g.glUniformMatrix4fv)(l, c, t, d) } }
        21 => (g.glEnableVertexAttribArray)(u(p, 0)),
        22 => (g.glDisableVertexAttribArray)(u(p, 0)),
        23 => (g.glVertexAttribPointer)(u(p, 0), s(p, 1), u(p, 2), u(p, 3) as u8, s(p, 4), u(p, 5) as usize as VP),
        74 => (g.glVertexAttrib4f)(u(p, 0), f(p, 1), f(p, 2), f(p, 3), f(p, 4)),
        25 => gen(st, g.glGenBuffers, u(p, 0)),
        26 => { if u(p, 0) == 0x8892 { st.bound_array = u(p, 1); } else if u(p, 0) == 0x8893 { st.bound_elem = u(p, 1); } (g.glBindBuffer)(u(p, 0), u(p, 1)); }
        27 => (g.glBufferData)(u(p, 0), u(p, 1) as isize, if u(p, 3) != 0 { p.add(4) as VP } else { null() }, u(p, 2)),
        28 => (g.glBufferSubData)(u(p, 0), u(p, 1) as isize, u(p, 2) as isize, p.add(3) as VP),
        53 => {
            // Keep client vertices in owned VBOs; the shared ring is reused after dispatch.
            let idx=u(p,0) as usize; let bytes=u(p,5) as usize;
            if idx>=16 || n<8 || bytes>(n-8)*4 {die(b"invalid client attribute payload\n");}
            if st.client_buffers[idx]==0 {(g.glGenBuffers)(1,&mut st.client_buffers[idx]);}
            (g.glBindBuffer)(0x8892,st.client_buffers[idx]);
            (g.glBufferData)(0x8892,bytes as isize,p.add(6) as VP,0x88E0);
            (g.glVertexAttribPointer)(idx as u32,s(p,1),u(p,2),u(p,3) as u8,s(p,4),null());
            (g.glBindBuffer)(0x8892,st.bound_array);
        }
        29 => (g.glDrawArrays)(u(p, 0), s(p, 1), s(p, 2)),
        30 => {
            if u(p, 4) != 0 {
                if st.bound_elem != 0 { (g.glBindBuffer)(0x8893, 0); }
                (g.glDrawElements)(u(p, 0), s(p, 1), u(p, 2), p.add(5) as VP);
                if st.bound_elem != 0 { (g.glBindBuffer)(0x8893, st.bound_elem); }
            } else { (g.glDrawElements)(u(p, 0), s(p, 1), u(p, 2), u(p, 3) as usize as VP); }
        }
        31 => gen(st, g.glGenTextures, u(p, 0)),
        32 => (g.glBindTexture)(u(p, 0), u(p, 1)),
        33 => (g.glTexImage2D)(u(p, 0), s(p, 1), s(p, 2), s(p, 3), s(p, 4), s(p, 5), u(p, 6), u(p, 7), if n - 2 > 8 { p.add(8) as VP } else { null() }),
        34 => (g.glTexSubImage2D)(u(p, 0), s(p, 1), s(p, 2), s(p, 3), s(p, 4), s(p, 5), u(p, 6), u(p, 7), p.add(8) as VP),
        35 => (g.glTexParameteri)(u(p, 0), u(p, 1), s(p, 2)),
        36 => (g.glActiveTexture)(u(p, 0)),
        37 => (g.glEnable)(u(p, 0)),
        38 => (g.glDisable)(u(p, 0)),
        39 => (g.glBlendFunc)(u(p, 0), u(p, 1)),
        40 => (g.glBlendFuncSeparate)(u(p, 0), u(p, 1), u(p, 2), u(p, 3)),
        41 => (g.glDepthFunc)(u(p, 0)),
        42 => (g.glDepthMask)(u(p, 0) as u8),
        43 => (g.glScissor)(s(p, 0), s(p, 1), s(p, 2), s(p, 3)),
        44 => (g.glPixelStorei)(u(p, 0), s(p, 1)),
        45 => { (g.glFinish)(); respond(&[0]); }
        46 => (g.glFrontFace)(u(p, 0)),
        47 => (g.glCullFace)(u(p, 0)),
        48 => {
            let id = u(p, 2);
            match u(p, 0) { 0 => (g.glDeleteShader)(id), 1 => (g.glDeleteProgram)(id), 2 => (g.glDeleteBuffers)(1, &id), 3 => (g.glDeleteTextures)(1, &id), 4 => (g.glDeleteFramebuffers)(1, &id), _ => (g.glDeleteRenderbuffers)(1, &id) }
        }
        49 => (g.glBlendEquationSeparate)(u(p, 0), u(p, 1)),
        50 => (g.glColorMask)(u(p, 0) as u8, u(p, 1) as u8, u(p, 2) as u8, u(p, 3) as u8),
        51 => (g.glGenerateMipmap)(u(p, 0)),
        52 => {
            let mut a = [0i32; 16]; (g.glGetIntegerv)(u(p, 0), a.as_mut_ptr());
            let cnt = match u(p, 0) { 0x0BA2 | 0x0C10 => 4, 0x0D3A | 0x846E | 0x846D => 2, _ => 1 };
            let mut r = [0u32; 17]; r[0] = cnt; let mut i = 0; while i < cnt as usize { r[1 + i] = a[i] as u32; i += 1; } respond(&r);
        }
        54 => { out(b"[BYE]\n"); }
        56 => (g.glBlendColor)(f(p, 0), f(p, 1), f(p, 2), f(p, 3)),
        57 => (g.glStencilFunc)(u(p, 0), s(p, 1), u(p, 2)),
        58 => (g.glStencilOp)(u(p, 0), u(p, 1), u(p, 2)),
        59 => (g.glStencilMask)(u(p, 0)),
        60 => (g.glClearDepthf)(f(p, 0)),
        61 => (g.glClearStencil)(s(p, 0)),
        62 => (g.glLineWidth)(f(p, 0)),
        63 => (g.glHint)(u(p, 0), u(p, 1)),
        64 => (g.glDepthRangef)(f(p, 0), f(p, 1)),
        65 => gen(st, g.glGenFramebuffers, u(p, 0)),
        66 => (g.glBindFramebuffer)(u(p, 0), u(p, 1)),
        67 => (g.glFramebufferTexture2D)(u(p, 0), u(p, 1), u(p, 2), u(p, 3), s(p, 4)),
        68 => gen(st, g.glGenRenderbuffers, u(p, 0)),
        69 => (g.glBindRenderbuffer)(u(p, 0), u(p, 1)),
        70 => (g.glRenderbufferStorage)(u(p, 0), u(p, 1), s(p, 2), s(p, 3)),
        71 => (g.glFramebufferRenderbuffer)(u(p, 0), u(p, 1), u(p, 2), u(p, 3)),
        72 => respond(&[compatible_framebuffer(g,u(p,0))]),
        73 => {
            let (row, h) = (u(p, 6) as usize, s(p, 3));
            if row * (h as usize) <= RESP_WORDS * 4 { (g.glReadPixels)(s(p, 0), s(p, 1), s(p, 2), h, u(p, 4), u(p, 5), resp() as *mut c_void); }
            bump_seq();
        }
        79 => (g.glPolygonOffset)(f(p, 0), f(p, 1)),
        80 => (g.glSampleCoverage)(f(p, 0), u(p, 1) as u8),
        81 => (g.glStencilFuncSeparate)(u(p, 0), u(p, 1), s(p, 2), u(p, 3)),
        82 => (g.glStencilOpSeparate)(u(p, 0), u(p, 1), u(p, 2), u(p, 3)),
        83 => (g.glStencilMaskSeparate)(u(p, 0), u(p, 1)),
        84 => (g.glCompressedTexImage2D)(u(p, 0), s(p, 1), u(p, 2), s(p, 3), s(p, 4), s(p, 5), s(p, 6), if u(p, 7) != 0 { p.add(8) as VP } else { null() }),
        85 => (g.glCompressedTexSubImage2D)(u(p, 0), s(p, 1), s(p, 2), s(p, 3), s(p, 4), s(p, 5), u(p, 6), s(p, 7), p.add(8) as VP),
        86 => (g.glCopyTexSubImage2D)(u(p, 0), s(p, 1), s(p, 2), s(p, 3), s(p, 4), s(p, 5), s(p, 6), s(p, 7)),
        87 | 88 => {
            let maxl = if u(p, 2) > 1000 { 1000 } else { u(p, 2) as i32 };
            let (mut l, mut sz, mut ty) = (0i32, 1i32, 0x1406u32);
            let nm = (resp() as *mut u8).add(12);
            if op == 87 { (g.glGetActiveUniform)(u(p, 0), u(p, 1), maxl, &mut l, &mut sz, &mut ty, nm); } else { (g.glGetActiveAttrib)(u(p, 0), u(p, 1), maxl, &mut l, &mut sz, &mut ty, nm); }
            write_volatile(resp(), l as u32); write_volatile(resp().add(1), sz as u32); write_volatile(resp().add(2), ty); bump_seq();
        }
        77 => (g.glCopyTexImage2D)(u(p, 0), s(p, 1), u(p, 2), s(p, 3), s(p, 4), s(p, 5), s(p, 6), s(p, 7)),
        _ => { out(b"lenh la "); out_num(op as u64); out(b"\n"); }
    }
    false
}
// PowerVR rejects separate depth/stencil renderbuffers. Upgrade that pair only
// when rejected, reusing the depth object so normal client deletion owns its lifetime.
unsafe fn compatible_framebuffer(g:&Gl,t:u32)->u32 {
    let original=(g.glCheckFramebufferStatus)(t);if original!=0x8CDD{return original;}
    let(mut dt,mut st,mut depth,mut stencil)=(0,0,0,0);
    (g.glGetFramebufferAttachmentParameteriv)(t,0x8D00,0x8CD0,&mut dt);
    (g.glGetFramebufferAttachmentParameteriv)(t,0x8D20,0x8CD0,&mut st);
    if dt!=0x8D41 || st!=0x8D41{return original;}
    (g.glGetFramebufferAttachmentParameteriv)(t,0x8D00,0x8CD1,&mut depth);
    (g.glGetFramebufferAttachmentParameteriv)(t,0x8D20,0x8CD1,&mut stencil);
    if depth==stencil{return original;}
    let(mut prev,mut dw,mut dh,mut df,mut sw,mut sh,mut sf)=(0,0,0,0,0,0,0);
    (g.glGetIntegerv)(0x8CA7,&mut prev);
    (g.glBindRenderbuffer)(0x8D41,depth as u32);
    (g.glGetRenderbufferParameteriv)(0x8D41,0x8D42,&mut dw);
    (g.glGetRenderbufferParameteriv)(0x8D41,0x8D43,&mut dh);
    (g.glGetRenderbufferParameteriv)(0x8D41,0x8D44,&mut df);
    (g.glBindRenderbuffer)(0x8D41,stencil as u32);
    (g.glGetRenderbufferParameteriv)(0x8D41,0x8D42,&mut sw);
    (g.glGetRenderbufferParameteriv)(0x8D41,0x8D43,&mut sh);
    (g.glGetRenderbufferParameteriv)(0x8D41,0x8D44,&mut sf);
    if dw<=0 || dh<=0 || dw!=sw || dh!=sh || (df!=0x81A5 && df!=0x81A6) || sf!=0x8D48 {
        (g.glBindRenderbuffer)(0x8D41,prev as u32);return original;
    }
    (g.glBindRenderbuffer)(0x8D41,depth as u32);
    (g.glRenderbufferStorage)(0x8D41,0x88F0,dw,dh);
    (g.glFramebufferRenderbuffer)(t,0x8D20,0x8D41,depth as u32);
    let result=(g.glCheckFramebufferStatus)(t);
    if result!=0x8CD5 {
        (g.glRenderbufferStorage)(0x8D41,df as u32,dw,dh);
        (g.glFramebufferRenderbuffer)(t,0x8D20,0x8D41,stencil as u32);
    } else {out(b"framebuffer: packed depth/stencil compatibility enabled\n");}
    (g.glBindRenderbuffer)(0x8D41,prev as u32);result
}

unsafe fn gen(_st: &mut St, fnp: unsafe extern "C" fn(i32, *mut u32), n: u32) {
    let mut ids = [0u32; 64]; let c = if n > 64 { 64 } else { n };
    fnp(c as i32, ids.as_mut_ptr()); respond(&ids[..c as usize]);
}


// ---- Tien trinh am thanh: `glserver --audio` doc vong PCM tu /tmp/glremote.snd (shim libasound ghi) va phat ra /dev/dsp (OSS) ----
const A_RING: usize = 262144; const A_FILE: usize = 64 + A_RING;
unsafe fn audio_main() -> ! {
    let path = b"/tmp/glremote.snd\0";
    let fd = sc(56, (-100isize) as usize, path.as_ptr() as usize, 0o2 | 0o100 | 0o1000, 0o666, 0, 0);
    if fd < 0 { die(b"audio: khong tao duoc tep\n"); }
    sc(46, fd as usize, A_FILE, 0, 0, 0, 0);
    let m = sc(222, 0, A_FILE, 3, 1, fd as usize, 0);
    if m < 0 && m > -4096 { die(b"audio: mmap loi\n"); }
    let hd = m as *mut u32; let ring = (m as *mut u8).add(64);
    for i in 0..16 { write_volatile(hd.add(i), 0); }
    write_volatile(hd.add(0), 0x444E5341); write_volatile(hd.add(7), 1);
    out(b"audio san sang\n");
    let mut dsp: isize = -1; let mut gen_seen = 0u32; let mut idle_ms = 0u64; let mut played: u64 = 0;
    loop {
        let active = read_volatile(hd.add(3)); let gen = read_volatile(hd.add(6));
        if active == 0 || gen != gen_seen {
            if dsp >= 0 { sc(57, dsp as usize, 0, 0, 0, 0, 0); dsp = -1; }
            if active == 0 { sleep_ns(20_000_000); idle_ms += 20; if idle_ms > 3_600_000 { break; } continue; }
            gen_seen = gen;
            let dev: &[u8] = if NULL_MODE { b"/dev/null\0" } else { b"/dev/dsp\0" };
            dsp = sc(56, (-100isize) as usize, dev.as_ptr() as usize, 1, 0, 0, 0);   // O_WRONLY
            if dsp < 0 { out(b"audio: khong mo duoc /dev/dsp\n"); sleep_ns(200_000_000); gen_seen = 0; continue; }
            let mut frag: u32 = (4 << 16) | 11; sc(29, dsp as usize, 0xC004500A, &mut frag as *mut u32 as usize, 0, 0, 0);
            let mut fmt: u32 = 0x10; sc(29, dsp as usize, 0xC0045005, &mut fmt as *mut u32 as usize, 0, 0, 0);       // AFMT_S16_LE
            let mut ch: u32 = read_volatile(hd.add(2)); sc(29, dsp as usize, 0xC0045006, &mut ch as *mut u32 as usize, 0, 0, 0);
            let mut sp: u32 = read_volatile(hd.add(1)); sc(29, dsp as usize, 0xC0045002, &mut sp as *mut u32 as usize, 0, 0, 0);
            out(b"audio: mo /dev/dsp, kenh="); out_num(ch as u64); out(b" toc do="); out_num(sp as u64); out(b"\n");
        }
        idle_ms = 0;
        let w = read_volatile(hd.add(4)); let r = read_volatile(hd.add(5));
        let avail = w.wrapping_sub(r) as usize;
        if avail < 512 { sleep_ns(1_000_000); continue; }
        fence(Ordering::Acquire);
        let off = (r as usize) % A_RING; let n = core::cmp::min(core::cmp::min(avail, A_RING - off), 4096);
        let wr = sc(64, dsp as usize, ring.add(off) as usize, n, 0, 0, 0);
        if wr > 0 { played += wr as u64; write_volatile(hd.add(5), r.wrapping_add(wr as u32)); }
        else { sleep_ns(2_000_000); }
    }
    out(b"audio: xong, byte="); out_num(played); out(b"\n");
    exit(0)
}
#[no_mangle]
pub unsafe extern "C" fn rust_main(sp: *const usize) -> ! {
    // tham số
    let argc = *sp; let argv = sp.add(1) as *const *const u8; let mut idle: u64 = 20_000;
    let mut i = 1; while i + 1 < argc {
        let a = *argv.add(i); let b = *argv.add(i + 1);
        if read_volatile(a) == b'-' && read_volatile(a.add(2)) == b's' { let mut v = 0u64; let mut k = 0; while read_volatile(b.add(k)) >= b'0' && read_volatile(b.add(k)) <= b'9' { v = v * 10 + (read_volatile(b.add(k)) - b'0') as u64; k += 1; } idle = v * 1000; }
        i += 1;
    }
    NULL_MODE = has_arg(argc, argv, b"--null");
    { let mut j = 1; while j < argc { let a = *argv.add(j); if read_volatile(a) == b'-' && read_volatile(a.add(2)) == b'a' { audio_main(); } j += 1; } }
    // bộ nhớ chung
    let path = b"/tmp/glremote.shm\0";
    let fd = sc(56, (-100isize) as usize, path.as_ptr() as usize, 0o2 | 0o100 | 0o1000, 0o666, 0, 0);   // O_RDWR|O_CREAT|O_TRUNC
    if fd < 0 { die(b"khong tao duoc shm\n"); }
    sc(46, fd as usize, SHM_BYTES, 0, 0, 0, 0);
    let m = sc(222, 0, SHM_BYTES, 3, 1, fd as usize, 0);
    if m < 0 && m > -4096 { die(b"mmap loi\n"); }
    SHM = m as *mut u32;
    // SDL + GLES
    let (mut win, mut ctx): (*mut c_void, *mut c_void) = (null_mut(), null_mut());
    if !NULL_MODE {
    if SDL_Init(0x20) != 0 { out(b"SDL_Init loi: "); out_cstr(SDL_GetError()); die(b"\n"); }
    SDL_GL_SetAttribute(17, 2); SDL_GL_SetAttribute(18, 0); SDL_GL_SetAttribute(21, 4);
    win = SDL_CreateWindow(b"glremote\0".as_ptr(), 0, 0, 1024, 768, 0x2 | 0x1 | 0x4);
    if win.is_null() { out(b"CreateWindow loi: "); out_cstr(SDL_GetError()); die(b"\n"); }
    ctx = SDL_GL_CreateContext(win);
    if ctx.is_null() { out(b"CreateContext loi: "); out_cstr(SDL_GetError()); die(b"\n"); }
    SDL_GL_SetSwapInterval(1);
    }
    let gl = if NULL_MODE { null_gl() } else { load_gl() };
    let rend = (gl.glGetString)(0x1F01);
    out(b"GL_RENDERER: "); out_cstr(rend); out(b" | VERSION: "); out_cstr((gl.glGetString)(0x1F02)); out(b"\n");
    let mut k = 0; while k < 250 && !rend.is_null() && read_volatile(rend.add(k)) != 0 { write_volatile((SHM.add(H_STR) as *mut u8).add(k), read_volatile(rend.add(k))); k += 1; }
    write_volatile((SHM.add(H_STR) as *mut u8).add(k), 0);
    write_volatile(hdr(H_WIDTH), 1024); write_volatile(hdr(H_HEIGHT), 768);
    write_volatile(hdr(H_RPOS), 0); write_volatile(hdr(H_WPOS), 0); write_volatile(hdr(H_RESP_SEQ), 0);
    fence(Ordering::Release);
    write_volatile(hdr(H_MAGIC), MAGIC); write_volatile(hdr(H_READY), 1);
    let mut st = St { gl, win, bound_array: 0, bound_elem: 0, client_buffers: [0;16], frames: 0, cmds: 0 };
    ST = &mut st;
    out(b"san sang, cho client (/tmp/glremote.shm)\n");

    let mask = RING_WORDS - 1;
    let mut r: u32 = 0; let mut t_last = now_ms(); let mut t_start = 0u64; let mut spins = 0u32; let mut bye = false; let mut t_report = 0u64; let mut last_op = 0u32;
    while !bye {
        let w = read_volatile(hdr(H_WPOS));
        if r == w {
            spins += 1;
            if spins > 4000 { sleep_ns(200_000); }
            if spins & 0x3FF == 0 && now_ms() - t_last > idle { out(b"het thoi gian cho\n"); break; }
            continue;
        }
        spins = 0; t_last = now_ms(); if t_start == 0 { t_start = t_last; }
        if t_last - t_report >= 1000 { t_report = t_last; out(b"[t+"); out_num((t_last - t_start) / 1000); out(b"s] khung="); out_num(st.frames); out(b" lenh="); out_num(st.cmds); out(b" lenh_cuoi="); out_num(last_op as u64); out(b"\n"); }
        fence(Ordering::Acquire);
        let mut cnt = 0u32;
        while r != w {
            let idx = (r as usize) & mask;
            let op = read_volatile(ring().add(idx)); let n = read_volatile(ring().add(idx + 1));
            if op == 0xFFFF { r = r.wrapping_add(n); continue; }
            last_op = op; if st.cmds < 80 { out(b"op "); out_num(op as u64); out(b"\n"); }
            bye = dispatch(&mut st, op, ring().add(idx + 2), n as usize);
            st.cmds += 1; r = r.wrapping_add(n); cnt += 1;
            if cnt & 63 == 0 { fence(Ordering::Release); write_volatile(hdr(H_RPOS), r); }
            if bye { break; }
        }
        fence(Ordering::Release); write_volatile(hdr(H_RPOS), r);
    }
    let dt = if t_start == 0 { 1 } else { now_ms() - t_start };
    out(b"xong: "); out_num(st.frames); out(b" khung, "); out_num(st.cmds); out(b" lenh trong "); out_num(dt); out(b" ms => FPSx10 ");
    out_num(st.frames * 10000 / if dt == 0 { 1 } else { dt }); out(b"\n");
    if !NULL_MODE { SDL_GL_DeleteContext(ctx); SDL_DestroyWindow(win); SDL_Quit(); }
    let path2 = b"/tmp/glremote.shm\0"; sc(35, (-100isize) as usize, path2.as_ptr() as usize, 0, 0, 0, 0);
    exit(0)
}




