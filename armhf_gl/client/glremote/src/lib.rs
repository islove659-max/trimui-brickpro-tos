//! glremote â€” shim EGL + OpenGL ES 2 cho á»©ng dá»¥ng ARMHF (32-bit), KHÃ”NG dÃ¹ng libc.
//! KhÃ´ng cÃ³ driver GPU 32-bit trÃªn Brick Pro, nÃªn má»i lá»‡nh Ä‘Æ°á»£c tuáº§n tá»± hoÃ¡ vÃ o bá»™ nhá»› chung
//! /tmp/glremote.shm rá»“i má»™t mÃ¡y chá»§ 64-bit (gl_server.py) cháº¡y chÃºng trÃªn PowerVR GE8300 tháº­t.
//! Giao thá»©c: vÃ²ng Ä‘á»‡m lá»‡nh (client ghi, server Ä‘á»c) + vÃ¹ng pháº£n há»“i cho lá»‡nh Ä‘á»“ng bá»™. Xem docs/ARMHF_GLREMOTE.md.
#![no_std]
#![allow(non_snake_case, non_upper_case_globals, clippy::missing_safety_doc)]

use core::arch::asm;
use core::panic::PanicInfo;
use core::ptr::{null, null_mut, read_volatile, write_volatile};
use core::sync::atomic::{fence, Ordering};

#[panic_handler]
fn panic(_: &PanicInfo) -> ! { loop {} }

// ARM EHABI: tÃªn hÃ m personality Ä‘Æ°á»£c tham chiáº¿u; khÃ´ng dÃ¹ng libc nÃªn tá»± cung cáº¥p báº£n rá»—ng (panic=abort).
#[no_mangle] pub extern "C" fn __aeabi_unwind_cpp_pr0() {}
#[no_mangle] pub extern "C" fn __aeabi_unwind_cpp_pr1() {}
#[no_mangle] pub extern "C" fn __aeabi_unwind_cpp_pr2() {}

// ---- hÃ m bá»™ nhá»› (compiler_builtins khÃ´ng kÃ¨m memcpy trÃªn target nÃ y); áº©n khá»i báº£ng xuáº¥t báº±ng exports.map ----
#[no_mangle] pub unsafe extern "C" fn memcpy(d: *mut u8, s: *const u8, n: usize) -> *mut u8 {
    let mut i = 0; while i < n { write_volatile(d.add(i), read_volatile(s.add(i))); i += 1; } d }
#[no_mangle] pub unsafe extern "C" fn memmove(d: *mut u8, s: *const u8, n: usize) -> *mut u8 {
    if (d as usize) < (s as usize) { memcpy(d, s, n) } else { let mut i = n; while i > 0 { i -= 1; write_volatile(d.add(i), read_volatile(s.add(i))); } d } }
#[no_mangle] pub unsafe extern "C" fn memset(d: *mut u8, c: i32, n: usize) -> *mut u8 {
    let mut i = 0; while i < n { write_volatile(d.add(i), c as u8); i += 1; } d }
#[no_mangle] pub unsafe extern "C" fn memcmp(a: *const u8, b: *const u8, n: usize) -> i32 {
    let mut i = 0; while i < n { let (x, y) = (read_volatile(a.add(i)), read_volatile(b.add(i))); if x != y { return x as i32 - y as i32; } i += 1; } 0 }
#[no_mangle] pub unsafe extern "C" fn bcmp(a: *const u8, b: *const u8, n: usize) -> i32 { memcmp(a, b, n) }

// ---- lá»i gá»i há»‡ thá»‘ng ARM EABI ----
#[inline(always)]
unsafe fn sc6(n: u32, a: u32, b: u32, c: u32, d: u32, e: u32, f: u32) -> i32 {
    let r: i32;
    asm!("svc 0", in("r7") n, inlateout("r0") a => r, in("r1") b, in("r2") c, in("r3") d, in("r4") e, in("r5") f, options(nostack));
    r
}
const SYS_OPEN: u32 = 5; const SYS_CLOSE: u32 = 6; const SYS_NANOSLEEP: u32 = 162; const SYS_YIELD: u32 = 158; const SYS_MMAP2: u32 = 192;
unsafe fn sys_yield() { sc6(SYS_YIELD, 0, 0, 0, 0, 0, 0); }
unsafe fn sys_sleep_us(us: u32) { let ts = [0u32, us * 1000]; sc6(SYS_NANOSLEEP, ts.as_ptr() as u32, 0, 0, 0, 0, 0); }

// ---- bá»‘ cá»¥c bá»™ nhá»› chung (KHá»šP vá»›i gl_server.py) ----
const RING_WORDS: usize = 1 << 20;            // 4 MiB
const HDR_WORDS: usize = 1024;                // 4 KiB
const RESP_WORDS: usize = 16384;              // 64 KiB
const SHM_BYTES: usize = (HDR_WORDS + RING_WORDS + RESP_WORDS) * 4;
const H_MAGIC: usize = 0; const H_READY: usize = 2; const H_RPOS: usize = 4; const H_WPOS: usize = 5;
const H_RESP_SEQ: usize = 6; const H_WIDTH: usize = 8; const H_HEIGHT: usize = 9;
const H_STR_RENDERER: usize = 64;             // chuá»—i C (NUL) 256 byte
const MAGIC: u32 = 0x4D524C47;                // "GLRM"
const OP_PAD: u32 = 0xFFFF;

static mut SHM: *mut u32 = null_mut();
static mut W: u32 = 0;                        // con trá» ghi (Ä‘Æ¡n vá»‹ tá»«, tÄƒng Ä‘Æ¡n Ä‘iá»‡u)
static mut SYNCN: u32 = 0;
static mut ERR: u32 = 0;

#[inline(always)] unsafe fn hdr(i: usize) -> *mut u32 { SHM.add(i) }
#[inline(always)] unsafe fn ring() -> *mut u32 { SHM.add(HDR_WORDS) }
#[inline(always)] unsafe fn resp() -> *mut u32 { SHM.add(HDR_WORDS + RING_WORDS) }

unsafe fn connect() -> bool {
    if !SHM.is_null() { return true; }
    let path = b"/tmp/glremote.shm\0";
    let mut tries = 0;
    let fd = loop {
        let fd = sc6(SYS_OPEN, path.as_ptr() as u32, 2, 0, 0, 0, 0);   // O_RDWR
        if fd >= 0 { break fd; }
        tries += 1; if tries > 200 { return false; }
        sys_sleep_us(50_000);
    };
    let p = sc6(SYS_MMAP2, 0, SHM_BYTES as u32, 3, 1, fd as u32, 0);   // PROT_READ|WRITE, MAP_SHARED
    sc6(SYS_CLOSE, fd as u32, 0, 0, 0, 0, 0);
    if (p as u32) > 0xFFFF_F000 { return false; }
    SHM = p as *mut u32;
    let mut t = 0;
    while read_volatile(hdr(H_MAGIC)) != MAGIC || read_volatile(hdr(H_READY)) == 0 { t += 1; if t > 400 { return false; } sys_sleep_us(25_000); }
    W = read_volatile(hdr(H_WPOS));
    SYNCN = read_volatile(hdr(H_RESP_SEQ));
    true
}

unsafe fn wait_space(need: u32) {
    let mut spins = 0u32;
    loop {
        let r = read_volatile(hdr(H_RPOS));
        if (RING_WORDS as u32).wrapping_sub(W.wrapping_sub(r)) >= need { return; }
        spins += 1; if spins > 64 { sys_yield(); }
    }
}

/// DÃ nh chá»— cho má»™t báº£n ghi `nwords` tá»« (ká»ƒ cáº£ 2 tá»« tiÃªu Ä‘á»). Tráº£ vá» con trá» tá»›i tá»« Ä‘áº§u báº£n ghi.
unsafe fn begin(nwords: usize) -> *mut u32 {
    W = read_volatile(hdr(H_WPOS));   // vị trí ghi dùng chung giữa các bản sao libEGL/libGLESv2
    let mask = RING_WORDS - 1;
    let idx = (W as usize) & mask;
    if idx + nwords > RING_WORDS {
        let pad = RING_WORDS - idx;
        wait_space((pad + nwords) as u32);
        let p = ring().add(idx);
        write_volatile(p, OP_PAD); write_volatile(p.add(1), pad as u32);
        fence(Ordering::Release);
        W = W.wrapping_add(pad as u32);
        write_volatile(hdr(H_WPOS), W);
    } else {
        wait_space(nwords as u32);
    }
    ring().add((W as usize) & mask)
}
unsafe fn commit(p: *mut u32, op: u32, nwords: usize) {
    write_volatile(p, op); write_volatile(p.add(1), nwords as u32);
    fence(Ordering::Release);
    W = W.wrapping_add(nwords as u32);
    write_volatile(hdr(H_WPOS), W);
}

/// Gá»­i lá»‡nh khÃ´ng Ä‘á»“ng bá»™: op + cÃ¡c tá»« tham sá»‘.
unsafe fn send(op: u32, args: &[u32]) {
    if SHM.is_null() && !connect() { return; }
    let n = 2 + args.len();
    let p = begin(n);
    let mut i = 0; while i < args.len() { write_volatile(p.add(2 + i), args[i]); i += 1; }
    commit(p, op, n);
}
/// Gá»­i lá»‡nh kÃ¨m khá»‘i dá»¯ liá»‡u (byte). `args` Ä‘á»©ng trÆ°á»›c, rá»“i `len` byte.
unsafe fn send_blob(op: u32, args: &[u32], data: *const u8, len: usize) {
    if SHM.is_null() && !connect() { return; }
    let dw = (len + 3) / 4;
    let n = 2 + args.len() + dw;
    let p = begin(n);
    let mut i = 0; while i < args.len() { write_volatile(p.add(2 + i), args[i]); i += 1; }
    if len > 0 && !data.is_null() { memcpy(p.add(2 + args.len()) as *mut u8, data, len); }
    commit(p, op, n);
}
/// Gá»i Ä‘á»“ng bá»™: gá»­i rá»“i chá» mÃ¡y chá»§ ghi pháº£n há»“i; tráº£ vá» con trá» vÃ¹ng pháº£n há»“i.
unsafe fn call(op: u32, args: &[u32]) -> *const u32 {
    if SHM.is_null() && !connect() { return null(); }
    SYNCN = read_volatile(hdr(H_RESP_SEQ));
    send(op, args);
    wait_resp();
    resp()
}
unsafe fn call_blob(op: u32, args: &[u32], data: *const u8, len: usize) -> *const u32 {
    if SHM.is_null() && !connect() { return null(); }
    SYNCN = read_volatile(hdr(H_RESP_SEQ));
    send_blob(op, args, data, len);
    wait_resp();
    resp()
}
unsafe fn wait_resp() {
    SYNCN = SYNCN.wrapping_add(1);
    let mut spins = 0u32;
    while read_volatile(hdr(H_RESP_SEQ)) != SYNCN { spins += 1; if spins > 256 { sys_yield(); } }
    fence(Ordering::Acquire);
}

// ---- mÃ£ lá»‡nh (KHá»šP gl_server.py) ----
const OP_HELLO: u32 = 1; const OP_SWAP: u32 = 2; const OP_CLEAR_COLOR: u32 = 3; const OP_CLEAR: u32 = 4; const OP_VIEWPORT: u32 = 5;
const OP_CREATE_SHADER: u32 = 6; const OP_SHADER_SOURCE: u32 = 7; const OP_COMPILE_SHADER: u32 = 8; const OP_GET_SHADER_IV: u32 = 9;
const OP_GET_SHADER_LOG: u32 = 10; const OP_CREATE_PROGRAM: u32 = 11; const OP_ATTACH_SHADER: u32 = 12; const OP_LINK_PROGRAM: u32 = 13;
const OP_GET_PROGRAM_IV: u32 = 14; const OP_USE_PROGRAM: u32 = 15; const OP_GET_ATTRIB_LOC: u32 = 16; const OP_GET_UNIFORM_LOC: u32 = 17;
const OP_UNIFORM_F: u32 = 18; const OP_UNIFORM_I: u32 = 19; const OP_UNIFORM_MAT: u32 = 20; const OP_ENABLE_VAA: u32 = 21; const OP_DISABLE_VAA: u32 = 22;
const OP_VAP_BUFFER: u32 = 23; const OP_GEN_BUFFERS: u32 = 25; const OP_BIND_BUFFER: u32 = 26; const OP_BUFFER_DATA: u32 = 27;
const OP_BUFFER_SUBDATA: u32 = 28; const OP_DRAW_ARRAYS: u32 = 29; const OP_DRAW_ELEMENTS: u32 = 30; const OP_GEN_TEXTURES: u32 = 31;
const OP_BIND_TEXTURE: u32 = 32; const OP_TEX_IMAGE: u32 = 33; const OP_TEX_SUBIMAGE: u32 = 34; const OP_TEX_PARAM_I: u32 = 35;
const OP_ACTIVE_TEXTURE: u32 = 36; const OP_ENABLE: u32 = 37; const OP_DISABLE: u32 = 38; const OP_BLEND_FUNC: u32 = 39;
const OP_BLEND_FUNC_SEP: u32 = 40; const OP_DEPTH_FUNC: u32 = 41; const OP_DEPTH_MASK: u32 = 42; const OP_SCISSOR: u32 = 43;
const OP_PIXEL_STORE: u32 = 44; const OP_FINISH: u32 = 45; const OP_FRONT_FACE: u32 = 46; const OP_CULL_FACE: u32 = 47;
const OP_DELETE_OBJS: u32 = 48; const OP_BLEND_EQ: u32 = 49; const OP_COLOR_MASK: u32 = 50; const OP_GEN_MIPMAP: u32 = 51;
const OP_GET_INTEGERV: u32 = 52; const OP_UPLOAD_ATTRIB: u32 = 53; const OP_BYE: u32 = 54; const OP_UNIFORM_FV: u32 = 55;
const OP_BLEND_COLOR: u32 = 56; const OP_STENCIL_FUNC: u32 = 57; const OP_STENCIL_OP: u32 = 58; const OP_STENCIL_MASK: u32 = 59;
const OP_CLEAR_DEPTH: u32 = 60; const OP_CLEAR_STENCIL: u32 = 61; const OP_LINE_WIDTH: u32 = 62; const OP_HINT: u32 = 63;
const OP_DEPTH_RANGE: u32 = 64; const OP_GEN_FRAMEBUFFERS: u32 = 65; const OP_BIND_FRAMEBUFFER: u32 = 66; const OP_FB_TEXTURE2D: u32 = 67;
const OP_GEN_RENDERBUFFERS: u32 = 68; const OP_BIND_RENDERBUFFER: u32 = 69; const OP_RB_STORAGE: u32 = 70; const OP_FB_RENDERBUFFER: u32 = 71;
const OP_CHECK_FB_STATUS: u32 = 72; const OP_READ_PIXELS: u32 = 73; const OP_VAP_CONST: u32 = 74; const OP_BIND_ATTRIB_LOC: u32 = 75;
const OP_TEX_PARAM_F: u32 = 76; const OP_COPY_TEX_IMAGE: u32 = 77; const OP_GET_ACTIVE: u32 = 78;

// ---- tráº¡ng thÃ¡i phÃ­a client ----
#[derive(Clone, Copy)]
struct Attrib { enabled: bool, size: u32, ty: u32, norm: u32, stride: u32, ptr: u32, buffer: u32 }
const A0: Attrib = Attrib { enabled: false, size: 4, ty: 0x1406, norm: 0, stride: 0, ptr: 0, buffer: 0 };
static mut ATTRIBS: [Attrib; 16] = [A0; 16];
static mut BOUND_ARRAY: u32 = 0;
static mut BOUND_ELEMENT: u32 = 0;
static mut UNPACK_ALIGN: u32 = 4;
static mut PACK_ALIGN: u32 = 4;
static mut SCRATCH: [u8; 65536] = [0; 65536];

fn type_size(t: u32) -> u32 { match t { 0x1400 | 0x1401 => 1, 0x1402 | 0x1403 | 0x140B => 2, 0x1404 | 0x1405 | 0x1406 | 0x140C => 4, _ => 4 } }
fn tex_bpp(fmt: u32, ty: u32) -> u32 {
    match ty {
        0x8033 | 0x8034 | 0x8363 => 2,                       // 4444 / 5551 / 565
        _ => { let ch = match fmt { 0x1906 | 0x1909 | 0x1903 => 1, 0x190A => 2, 0x1907 => 3, 0x1908 | 0x80E1 => 4, _ => 4 }; ch * type_size(ty) }
    }
}
unsafe fn cstrlen(p: *const u8) -> usize { let mut n = 0; while read_volatile(p.add(n)) != 0 { n += 1; } n }

// =====================================================================================
// EGL (pháº§n lá»›n lÃ  giáº£ láº­p phÃ­a client; chá»‰ Initialize/Swap nÃ³i chuyá»‡n vá»›i mÃ¡y chá»§)
// =====================================================================================
pub type EGLint = i32;
pub type EGLBoolean = u32;
const EGL_TRUE: u32 = 1; const EGL_FALSE: u32 = 0;
const DPY: usize = 0x1; const CFG: usize = 0x2; const CTX: usize = 0x3; const SURF: usize = 0x4;
static mut SCR_W: u32 = 1024; static mut SCR_H: u32 = 768;

#[no_mangle] pub unsafe extern "C" fn eglGetDisplay(_d: usize) -> usize { trace(b"eglGetDisplay", null()); DPY }
#[no_mangle] pub unsafe extern "C" fn eglInitialize(_d: usize, major: *mut i32, minor: *mut i32) -> EGLBoolean {
    trace(b"eglInitialize", null());
    if !connect() { ERR = 0x3001; return EGL_FALSE; }
    let r = call(OP_HELLO, &[1]);
    if !r.is_null() { SCR_W = read_volatile(r); SCR_H = read_volatile(r.add(1)); }
    if !major.is_null() { *major = 1; } if !minor.is_null() { *minor = 4; }
    EGL_TRUE
}
#[no_mangle] pub unsafe extern "C" fn eglTerminate(_d: usize) -> EGLBoolean { send(OP_BYE, &[]); EGL_TRUE }
#[no_mangle] pub unsafe extern "C" fn eglGetError() -> EGLint { let e = ERR; ERR = 0; if e == 0 { 0x3000 } else { e as i32 } }
#[no_mangle] pub unsafe extern "C" fn eglBindAPI(a: u32) -> EGLBoolean { trace(b"eglBindAPI", null()); if a == 0x30A0 { EGL_TRUE } else { ERR = 0x300C; EGL_FALSE } }
#[no_mangle] pub unsafe extern "C" fn eglQueryAPI() -> u32 { 0x30A0 }
#[no_mangle] pub unsafe extern "C" fn eglChooseConfig(_d: usize, _attr: *const i32, configs: *mut usize, size: i32, num: *mut i32) -> EGLBoolean {
    trace(b"eglChooseConfig", null());
    if !configs.is_null() && size > 0 { *configs = CFG; }
    if !num.is_null() { *num = 1; }
    EGL_TRUE
}
#[no_mangle] pub unsafe extern "C" fn eglGetConfigs(_d: usize, configs: *mut usize, size: i32, num: *mut i32) -> EGLBoolean { eglChooseConfig(_d, null(), configs, size, num) }
#[no_mangle] pub unsafe extern "C" fn eglGetConfigAttrib(_d: usize, _c: usize, attr: i32, value: *mut i32) -> EGLBoolean {
    let v = match attr {
        0x3020 => 32, 0x3024 | 0x3023 | 0x3022 | 0x3021 => 8, 0x3025 => 24, 0x3026 => 8, 0x3028 => 1,
        0x3033 => 5, 0x3040 => 0x44, 0x3042 => 0x44, 0x302E => 0x34325258, 0x3027 => 0x3038, 0x3031 | 0x3032 => 0, 0x3029 | 0x302A => 0,
        0x302B | 0x302C => 0, 0x3039 => 0, 0x303B => 4096, 0x303C => 4096, 0x303A => 4096,
        _ => 0,
    };
    if !value.is_null() { *value = v; }
    EGL_TRUE
}
#[no_mangle] pub unsafe extern "C" fn eglCreateWindowSurface(_d: usize, _c: usize, _w: usize, _a: *const i32) -> usize { trace(b"eglCreateWindowSurface", null()); SURF }
#[no_mangle] pub unsafe extern "C" fn eglCreatePbufferSurface(_d: usize, _c: usize, _a: *const i32) -> usize { SURF }
#[no_mangle] pub unsafe extern "C" fn eglCreatePixmapSurface(_d: usize, _c: usize, _p: usize, _a: *const i32) -> usize { SURF }
#[no_mangle] pub unsafe extern "C" fn eglDestroySurface(_d: usize, _s: usize) -> EGLBoolean { EGL_TRUE }
#[no_mangle] pub unsafe extern "C" fn eglCreateContext(_d: usize, _c: usize, _s: usize, _a: *const i32) -> usize { trace(b"eglCreateContext", null()); CTX }
#[no_mangle] pub unsafe extern "C" fn eglDestroyContext(_d: usize, _c: usize) -> EGLBoolean { EGL_TRUE }
#[no_mangle] pub unsafe extern "C" fn eglMakeCurrent(_d: usize, _dr: usize, _rd: usize, _c: usize) -> EGLBoolean { trace(b"eglMakeCurrent", null()); EGL_TRUE }
#[no_mangle] pub unsafe extern "C" fn eglGetCurrentContext() -> usize { CTX }
#[no_mangle] pub unsafe extern "C" fn eglGetCurrentSurface(_r: i32) -> usize { SURF }
#[no_mangle] pub unsafe extern "C" fn eglGetCurrentDisplay() -> usize { DPY }
#[no_mangle] pub unsafe extern "C" fn eglSwapInterval(_d: usize, _i: i32) -> EGLBoolean { EGL_TRUE }
#[no_mangle] pub unsafe extern "C" fn eglWaitGL() -> EGLBoolean { EGL_TRUE }
#[no_mangle] pub unsafe extern "C" fn eglWaitNative(_e: i32) -> EGLBoolean { EGL_TRUE }
#[no_mangle] pub unsafe extern "C" fn eglReleaseThread() -> EGLBoolean { EGL_TRUE }
#[no_mangle] pub unsafe extern "C" fn eglSwapBuffers(_d: usize, _s: usize) -> EGLBoolean { call(OP_SWAP, &[]); EGL_TRUE }
#[no_mangle] pub unsafe extern "C" fn eglQuerySurface(_d: usize, _s: usize, attr: i32, value: *mut i32) -> EGLBoolean {
    let v = match attr { 0x3057 => SCR_W as i32, 0x3056 => SCR_H as i32, 0x3086 => 0x3084, 0x3090 => 0x3085, 0x3093 => 0, _ => 0 };
    if !value.is_null() { *value = v; } EGL_TRUE
}
#[no_mangle] pub unsafe extern "C" fn eglQueryContext(_d: usize, _c: usize, attr: i32, value: *mut i32) -> EGLBoolean {
    if !value.is_null() { *value = if attr == 0x3098 { 2 } else { 0 }; } EGL_TRUE
}
#[no_mangle] pub unsafe extern "C" fn eglQueryString(_d: usize, name: i32) -> *const u8 {
    trace(b"eglQueryString", null());
    match name { 0x3053 => b"glremote\0".as_ptr(), 0x3054 => b"1.4 glremote\0".as_ptr(), 0x3055 => b"\0".as_ptr(), 0x308D => b"EGL_KHR_surfaceless_context\0".as_ptr(), _ => null() }
}
#[no_mangle] pub unsafe extern "C" fn eglSurfaceAttrib(_d: usize, _s: usize, _a: i32, _v: i32) -> EGLBoolean { EGL_TRUE }
#[no_mangle] pub unsafe extern "C" fn eglCopyBuffers(_d: usize, _s: usize, _p: usize) -> EGLBoolean { EGL_FALSE }

// =====================================================================================
// OpenGL ES 2.0 â€” lá»‡nh tráº¡ng thÃ¡i Ä‘Æ¡n giáº£n
// =====================================================================================
#[no_mangle] pub unsafe extern "C" fn glClearColor(r: f32, g: f32, b: f32, a: f32) { send(OP_CLEAR_COLOR, &[r.to_bits(), g.to_bits(), b.to_bits(), a.to_bits()]); }
#[no_mangle] pub unsafe extern "C" fn glClear(mask: u32) { send(OP_CLEAR, &[mask]); }
#[no_mangle] pub unsafe extern "C" fn glClearDepthf(d: f32) { send(OP_CLEAR_DEPTH, &[d.to_bits()]); }
#[no_mangle] pub unsafe extern "C" fn glClearStencil(s: i32) { send(OP_CLEAR_STENCIL, &[s as u32]); }
#[no_mangle] pub unsafe extern "C" fn glViewport(x: i32, y: i32, w: i32, h: i32) { send(OP_VIEWPORT, &[x as u32, y as u32, w as u32, h as u32]); }
#[no_mangle] pub unsafe extern "C" fn glScissor(x: i32, y: i32, w: i32, h: i32) { send(OP_SCISSOR, &[x as u32, y as u32, w as u32, h as u32]); }
#[no_mangle] pub unsafe extern "C" fn glEnable(c: u32) { send(OP_ENABLE, &[c]); }
#[no_mangle] pub unsafe extern "C" fn glDisable(c: u32) { send(OP_DISABLE, &[c]); }
#[no_mangle] pub unsafe extern "C" fn glBlendFunc(s: u32, d: u32) { send(OP_BLEND_FUNC, &[s, d]); }
#[no_mangle] pub unsafe extern "C" fn glBlendFuncSeparate(a: u32, b: u32, c: u32, d: u32) { send(OP_BLEND_FUNC_SEP, &[a, b, c, d]); }
#[no_mangle] pub unsafe extern "C" fn glBlendEquation(m: u32) { send(OP_BLEND_EQ, &[m, m]); }
#[no_mangle] pub unsafe extern "C" fn glBlendEquationSeparate(a: u32, b: u32) { send(OP_BLEND_EQ, &[a, b]); }
#[no_mangle] pub unsafe extern "C" fn glBlendColor(r: f32, g: f32, b: f32, a: f32) { send(OP_BLEND_COLOR, &[r.to_bits(), g.to_bits(), b.to_bits(), a.to_bits()]); }
#[no_mangle] pub unsafe extern "C" fn glDepthFunc(f: u32) { send(OP_DEPTH_FUNC, &[f]); }
#[no_mangle] pub unsafe extern "C" fn glDepthMask(m: u8) { send(OP_DEPTH_MASK, &[m as u32]); }
#[no_mangle] pub unsafe extern "C" fn glDepthRangef(n: f32, f: f32) { send(OP_DEPTH_RANGE, &[n.to_bits(), f.to_bits()]); }
#[no_mangle] pub unsafe extern "C" fn glColorMask(r: u8, g: u8, b: u8, a: u8) { send(OP_COLOR_MASK, &[r as u32, g as u32, b as u32, a as u32]); }
#[no_mangle] pub unsafe extern "C" fn glFrontFace(m: u32) { send(OP_FRONT_FACE, &[m]); }
#[no_mangle] pub unsafe extern "C" fn glCullFace(m: u32) { send(OP_CULL_FACE, &[m]); }
#[no_mangle] pub unsafe extern "C" fn glLineWidth(w: f32) { send(OP_LINE_WIDTH, &[w.to_bits()]); }
#[no_mangle] pub unsafe extern "C" fn glHint(t: u32, m: u32) { send(OP_HINT, &[t, m]); }
#[no_mangle] pub unsafe extern "C" fn glStencilFunc(f: u32, r: i32, m: u32) { send(OP_STENCIL_FUNC, &[f, r as u32, m]); }
#[no_mangle] pub unsafe extern "C" fn glStencilOp(a: u32, b: u32, c: u32) { send(OP_STENCIL_OP, &[a, b, c]); }
#[no_mangle] pub unsafe extern "C" fn glStencilMask(m: u32) { send(OP_STENCIL_MASK, &[m]); }
#[no_mangle] pub unsafe extern "C" fn glPixelStorei(p: u32, v: i32) {
    if p == 0x0CF5 { UNPACK_ALIGN = v as u32; } else if p == 0x0D05 { PACK_ALIGN = v as u32; }
    send(OP_PIXEL_STORE, &[p, v as u32]);
}
#[no_mangle] pub unsafe extern "C" fn glFinish() { call(OP_FINISH, &[]); }
#[no_mangle] pub unsafe extern "C" fn glFlush() {}
#[no_mangle] pub unsafe extern "C" fn glGetError() -> u32 { 0 }   // khÃ´ng khá»© há»“i má»—i láº§n gá»i (nhanh); lá»—i tháº­t xem log cá»§a mÃ¡y chá»§
#[no_mangle] pub unsafe extern "C" fn glActiveTexture(t: u32) { send(OP_ACTIVE_TEXTURE, &[t]); }

// ---- chuá»—i / truy váº¥n ----
static VENDOR: &[u8] = b"glremote (Imagination Technologies PowerVR via 64-bit server)\0";
static VERSION: &[u8] = b"OpenGL ES 2.0 glremote\0";
static GLSL: &[u8] = b"OpenGL ES GLSL ES 1.00\0";
static EXT: &[u8] = b"GL_OES_rgb8_rgba8 GL_EXT_texture_format_BGRA8888\0";
#[no_mangle] pub unsafe extern "C" fn glGetString(name: u32) -> *const u8 {
    match name {
        0x1F00 => VENDOR.as_ptr(),
        0x1F01 => if SHM.is_null() && !connect() { b"glremote\0".as_ptr() } else { SHM.add(H_STR_RENDERER) as *const u8 },
        0x1F02 => VERSION.as_ptr(), 0x8B8C => GLSL.as_ptr(), 0x1F03 => EXT.as_ptr(), _ => null(),
    }
}
#[no_mangle] pub unsafe extern "C" fn glGetIntegerv(pname: u32, data: *mut i32) {
    let r = call(OP_GET_INTEGERV, &[pname]);
    if r.is_null() || data.is_null() { return; }
    let n = read_volatile(r) as usize; let mut i = 0; while i < n && i < 16 { *data.add(i) = read_volatile(r.add(1 + i)) as i32; i += 1; }
}
#[no_mangle] pub unsafe extern "C" fn glGetBooleanv(pname: u32, data: *mut u8) { let mut t = [0i32; 16]; glGetIntegerv(pname, t.as_mut_ptr()); if !data.is_null() { *data = (t[0] != 0) as u8; } }
#[no_mangle] pub unsafe extern "C" fn glGetFloatv(pname: u32, data: *mut f32) { let mut t = [0i32; 16]; glGetIntegerv(pname, t.as_mut_ptr()); if !data.is_null() { *data = t[0] as f32; } }
#[no_mangle] pub unsafe extern "C" fn glGetShaderPrecisionFormat(_s: u32, _p: u32, range: *mut i32, prec: *mut i32) {
    if !range.is_null() { *range = 127; *range.add(1) = 127; } if !prec.is_null() { *prec = 23; }
}
#[no_mangle] pub unsafe extern "C" fn glIsEnabled(_c: u32) -> u8 { 0 }

// ---- shader / program ----
#[no_mangle] pub unsafe extern "C" fn glCreateShader(t: u32) -> u32 { let r = call(OP_CREATE_SHADER, &[t]); if r.is_null() { 0 } else { read_volatile(r) } }
#[no_mangle] pub unsafe extern "C" fn glShaderSource(shader: u32, count: i32, strings: *const *const u8, lengths: *const i32) {
    let mut total = 0usize; let mut i = 0;
    while i < count as usize {
        let s = *strings.add(i); let l = if lengths.is_null() || *lengths.add(i) < 0 { cstrlen(s) } else { *lengths.add(i) as usize };
        if total + l < SCRATCH.len() { memcpy(SCRATCH.as_mut_ptr().add(total), s, l); total += l; }
        i += 1;
    }
    send_blob(OP_SHADER_SOURCE, &[shader, total as u32], SCRATCH.as_ptr(), total);
}
#[no_mangle] pub unsafe extern "C" fn glCompileShader(s: u32) { send(OP_COMPILE_SHADER, &[s]); }
#[no_mangle] pub unsafe extern "C" fn glGetShaderiv(s: u32, p: u32, out: *mut i32) { let r = call(OP_GET_SHADER_IV, &[s, p]); if !r.is_null() && !out.is_null() { *out = read_volatile(r) as i32; } }
unsafe fn copy_log(r: *const u32, max: i32, len: *mut i32, buf: *mut u8) {
    if r.is_null() { if !len.is_null() { *len = 0; } return; }
    let n = read_volatile(r) as i32; let m = if n < max - 1 { n } else { max - 1 }; let m = if m < 0 { 0 } else { m };
    if !buf.is_null() && max > 0 { memcpy(buf, r.add(1) as *const u8, m as usize); *buf.add(m as usize) = 0; }
    if !len.is_null() { *len = m; }
}
#[no_mangle] pub unsafe extern "C" fn glGetShaderInfoLog(s: u32, max: i32, len: *mut i32, buf: *mut u8) { let r = call(OP_GET_SHADER_LOG, &[s, 0]); copy_log(r, max, len, buf); }
#[no_mangle] pub unsafe extern "C" fn glGetProgramInfoLog(p: u32, max: i32, len: *mut i32, buf: *mut u8) { let r = call(OP_GET_SHADER_LOG, &[p, 1]); copy_log(r, max, len, buf); }
#[no_mangle] pub unsafe extern "C" fn glCreateProgram() -> u32 { let r = call(OP_CREATE_PROGRAM, &[]); if r.is_null() { 0 } else { read_volatile(r) } }
#[no_mangle] pub unsafe extern "C" fn glAttachShader(p: u32, s: u32) { send(OP_ATTACH_SHADER, &[p, s]); }
#[no_mangle] pub unsafe extern "C" fn glLinkProgram(p: u32) { send(OP_LINK_PROGRAM, &[p]); }
#[no_mangle] pub unsafe extern "C" fn glGetProgramiv(p: u32, pn: u32, out: *mut i32) { let r = call(OP_GET_PROGRAM_IV, &[p, pn]); if !r.is_null() && !out.is_null() { *out = read_volatile(r) as i32; } }
#[no_mangle] pub unsafe extern "C" fn glUseProgram(p: u32) { send(OP_USE_PROGRAM, &[p]); }
#[no_mangle] pub unsafe extern "C" fn glBindAttribLocation(p: u32, idx: u32, name: *const u8) { let l = cstrlen(name); send_blob(OP_BIND_ATTRIB_LOC, &[p, idx, l as u32], name, l); }
#[no_mangle] pub unsafe extern "C" fn glGetAttribLocation(p: u32, name: *const u8) -> i32 { let l = cstrlen(name); let r = call_blob(OP_GET_ATTRIB_LOC, &[p, l as u32], name, l); if r.is_null() { -1 } else { read_volatile(r) as i32 } }
#[no_mangle] pub unsafe extern "C" fn glGetUniformLocation(p: u32, name: *const u8) -> i32 { let l = cstrlen(name); let r = call_blob(OP_GET_UNIFORM_LOC, &[p, l as u32], name, l); if r.is_null() { -1 } else { read_volatile(r) as i32 } }
#[no_mangle] pub unsafe extern "C" fn glDeleteShader(s: u32) { send(OP_DELETE_OBJS, &[0, 1, s]); }
#[no_mangle] pub unsafe extern "C" fn glDeleteProgram(p: u32) { send(OP_DELETE_OBJS, &[1, 1, p]); }
#[no_mangle] pub unsafe extern "C" fn glShaderBinary(_n: i32, _s: *const u32, _f: u32, _b: *const u8, _l: i32) {}
#[no_mangle] pub unsafe extern "C" fn glReleaseShaderCompiler() {}
#[no_mangle] pub unsafe extern "C" fn glDetachShader(_p: u32, _s: u32) {}
#[no_mangle] pub unsafe extern "C" fn glIsTexture(_t: u32) -> u8 { 1 }
#[no_mangle] pub unsafe extern "C" fn glValidateProgram(_p: u32) {}

// ---- uniform ----
#[no_mangle] pub unsafe extern "C" fn glUniform1f(l: i32, a: f32) { send(OP_UNIFORM_F, &[l as u32, 1, a.to_bits()]); }
#[no_mangle] pub unsafe extern "C" fn glUniform2f(l: i32, a: f32, b: f32) { send(OP_UNIFORM_F, &[l as u32, 2, a.to_bits(), b.to_bits()]); }
#[no_mangle] pub unsafe extern "C" fn glUniform3f(l: i32, a: f32, b: f32, c: f32) { send(OP_UNIFORM_F, &[l as u32, 3, a.to_bits(), b.to_bits(), c.to_bits()]); }
#[no_mangle] pub unsafe extern "C" fn glUniform4f(l: i32, a: f32, b: f32, c: f32, d: f32) { send(OP_UNIFORM_F, &[l as u32, 4, a.to_bits(), b.to_bits(), c.to_bits(), d.to_bits()]); }
#[no_mangle] pub unsafe extern "C" fn glUniform1i(l: i32, a: i32) { send(OP_UNIFORM_I, &[l as u32, 1, a as u32]); }
#[no_mangle] pub unsafe extern "C" fn glUniform2i(l: i32, a: i32, b: i32) { send(OP_UNIFORM_I, &[l as u32, 2, a as u32, b as u32]); }
#[no_mangle] pub unsafe extern "C" fn glUniform3i(l: i32, a: i32, b: i32, c: i32) { send(OP_UNIFORM_I, &[l as u32, 3, a as u32, b as u32, c as u32]); }
#[no_mangle] pub unsafe extern "C" fn glUniform4i(l: i32, a: i32, b: i32, c: i32, d: i32) { send(OP_UNIFORM_I, &[l as u32, 4, a as u32, b as u32, c as u32, d as u32]); }
unsafe fn uniform_fv(n: u32, l: i32, count: i32, v: *const f32) { send_blob(OP_UNIFORM_FV, &[l as u32, n, count as u32], v as *const u8, (n * count as u32 * 4) as usize); }
#[no_mangle] pub unsafe extern "C" fn glUniform1fv(l: i32, c: i32, v: *const f32) { uniform_fv(1, l, c, v); }
#[no_mangle] pub unsafe extern "C" fn glUniform2fv(l: i32, c: i32, v: *const f32) { uniform_fv(2, l, c, v); }
#[no_mangle] pub unsafe extern "C" fn glUniform3fv(l: i32, c: i32, v: *const f32) { uniform_fv(3, l, c, v); }
#[no_mangle] pub unsafe extern "C" fn glUniform4fv(l: i32, c: i32, v: *const f32) { uniform_fv(4, l, c, v); }
#[no_mangle] pub unsafe extern "C" fn glUniformMatrix2fv(l: i32, c: i32, t: u8, v: *const f32) { send_blob(OP_UNIFORM_MAT, &[l as u32, 2, c as u32, t as u32], v as *const u8, (4 * c * 4) as usize); }
#[no_mangle] pub unsafe extern "C" fn glUniformMatrix3fv(l: i32, c: i32, t: u8, v: *const f32) { send_blob(OP_UNIFORM_MAT, &[l as u32, 3, c as u32, t as u32], v as *const u8, (9 * c * 4) as usize); }
#[no_mangle] pub unsafe extern "C" fn glUniformMatrix4fv(l: i32, c: i32, t: u8, v: *const f32) { send_blob(OP_UNIFORM_MAT, &[l as u32, 4, c as u32, t as u32], v as *const u8, (16 * c * 4) as usize); }

// ---- thuá»™c tÃ­nh Ä‘á»‰nh + buffer ----
#[no_mangle] pub unsafe extern "C" fn glEnableVertexAttribArray(i: u32) { if i < 16 { ATTRIBS[i as usize].enabled = true; } send(OP_ENABLE_VAA, &[i]); }
#[no_mangle] pub unsafe extern "C" fn glDisableVertexAttribArray(i: u32) { if i < 16 { ATTRIBS[i as usize].enabled = false; } send(OP_DISABLE_VAA, &[i]); }
#[no_mangle] pub unsafe extern "C" fn glVertexAttribPointer(i: u32, size: i32, ty: u32, norm: u8, stride: i32, ptr: usize) {
    if i >= 16 { return; }
    let a = &mut ATTRIBS[i as usize];
    a.size = size as u32; a.ty = ty; a.norm = norm as u32; a.stride = stride as u32; a.buffer = BOUND_ARRAY; a.ptr = ptr as u32;
    if BOUND_ARRAY != 0 { send(OP_VAP_BUFFER, &[i, size as u32, ty, norm as u32, stride as u32, ptr as u32]); }
}
#[no_mangle] pub unsafe extern "C" fn glVertexAttrib4f(i: u32, a: f32, b: f32, c: f32, d: f32) { send(OP_VAP_CONST, &[i, a.to_bits(), b.to_bits(), c.to_bits(), d.to_bits()]); }
#[no_mangle] pub unsafe extern "C" fn glVertexAttrib1f(i: u32, a: f32) { glVertexAttrib4f(i, a, 0.0, 0.0, 1.0); }
#[no_mangle] pub unsafe extern "C" fn glVertexAttrib2f(i: u32, a: f32, b: f32) { glVertexAttrib4f(i, a, b, 0.0, 1.0); }
#[no_mangle] pub unsafe extern "C" fn glVertexAttrib3f(i: u32, a: f32, b: f32, c: f32) { glVertexAttrib4f(i, a, b, c, 1.0); }
unsafe fn gen_objs(op: u32, n: i32, out: *mut u32) { if n <= 0 || out.is_null() { return; } let r = call(op, &[n as u32]); if r.is_null() { return; } let mut i = 0; while i < n as usize { *out.add(i) = read_volatile(r.add(i)); i += 1; } }
#[no_mangle] pub unsafe extern "C" fn glGenBuffers(n: i32, out: *mut u32) { gen_objs(OP_GEN_BUFFERS, n, out); }
#[no_mangle] pub unsafe extern "C" fn glBindBuffer(t: u32, id: u32) { if t == 0x8892 { BOUND_ARRAY = id; } else if t == 0x8893 { BOUND_ELEMENT = id; } send(OP_BIND_BUFFER, &[t, id]); }
#[no_mangle] pub unsafe extern "C" fn glBufferData(t: u32, size: isize, data: *const u8, usage: u32) { send_blob(OP_BUFFER_DATA, &[t, size as u32, usage, (!data.is_null()) as u32], data, if data.is_null() { 0 } else { size as usize }); }
#[no_mangle] pub unsafe extern "C" fn glBufferSubData(t: u32, off: isize, size: isize, data: *const u8) { send_blob(OP_BUFFER_SUBDATA, &[t, off as u32, size as u32], data, size as usize); }
#[no_mangle] pub unsafe extern "C" fn glDeleteBuffers(n: i32, ids: *const u32) { let mut i = 0; while i < n as usize { send(OP_DELETE_OBJS, &[2, 1, *ids.add(i)]); i += 1; } }

/// Äáº©y dá»¯ liá»‡u cÃ¡c máº£ng Ä‘á»‰nh náº±m trong bá»™ nhá»› client (khÃ´ng cÃ³ VBO) tá»›i mÃ¡y chá»§ trÆ°á»›c khi váº½.
/// `max_vertex` = chá»‰ sá»‘ Ä‘á»‰nh lá»›n nháº¥t Ä‘Æ°á»£c dÃ¹ng (Ä‘Ã£ cá»™ng 1 = sá»‘ Ä‘á»‰nh cáº§n).
unsafe fn upload_client_attribs(vertex_count: u32) {
    let mut i = 0;
    while i < 16 {
        let a = ATTRIBS[i];
        if a.enabled && a.buffer == 0 && a.ptr != 0 {
            let esz = a.size * type_size(a.ty);
            let stride = if a.stride == 0 { esz } else { a.stride };
            let bytes = if vertex_count == 0 { 0 } else { (vertex_count - 1) * stride + esz };
            send_blob(OP_UPLOAD_ATTRIB, &[i as u32, a.size, a.ty, a.norm, a.stride, bytes], a.ptr as *const u8, bytes as usize);
        }
        i += 1;
    }
}
#[no_mangle] pub unsafe extern "C" fn glDrawArrays(mode: u32, first: i32, count: i32) {
    if count <= 0 { return; }
    upload_client_attribs((first + count) as u32);
    send(OP_DRAW_ARRAYS, &[mode, first as u32, count as u32]);
}
#[no_mangle] pub unsafe extern "C" fn glDrawElements(mode: u32, count: i32, ty: u32, indices: usize) {
    if count <= 0 { return; }
    let isz = type_size(ty);
    if BOUND_ELEMENT != 0 {
        // chá»‰ sá»‘ náº±m trong VBO pháº§n tá»­; máº£ng Ä‘á»‰nh client váº«n cáº§n sá»‘ Ä‘á»‰nh tá»‘i Ä‘a => khÃ´ng biáº¿t, dÃ¹ng quÃ©t khÃ´ng Ä‘Æ°á»£c: yÃªu cáº§u VBO cho attrib.
        send(OP_DRAW_ELEMENTS, &[mode, count as u32, ty, indices as u32, 0]);
        return;
    }
    // chá»‰ sá»‘ trong bá»™ nhá»› client: tÃ¬m chá»‰ sá»‘ lá»›n nháº¥t Ä‘á»ƒ biáº¿t cáº§n Ä‘áº©y bao nhiÃªu Ä‘á»‰nh
    let mut maxi = 0u32; let mut j = 0usize; let p = indices as *const u8;
    while j < count as usize {
        let v = match isz { 1 => *p.add(j) as u32, 2 => (*(p.add(j * 2) as *const u16)) as u32, _ => *(p.add(j * 4) as *const u32) };
        if v > maxi { maxi = v; } j += 1;
    }
    upload_client_attribs(maxi + 1);
    send_blob(OP_DRAW_ELEMENTS, &[mode, count as u32, ty, 0, 1], p, (count as u32 * isz) as usize);
}

// ---- texture ----
#[no_mangle] pub unsafe extern "C" fn glGenTextures(n: i32, out: *mut u32) { gen_objs(OP_GEN_TEXTURES, n, out); }
#[no_mangle] pub unsafe extern "C" fn glBindTexture(t: u32, id: u32) { send(OP_BIND_TEXTURE, &[t, id]); }
#[no_mangle] pub unsafe extern "C" fn glDeleteTextures(n: i32, ids: *const u32) { let mut i = 0; while i < n as usize { send(OP_DELETE_OBJS, &[3, 1, *ids.add(i)]); i += 1; } }
#[no_mangle] pub unsafe extern "C" fn glTexParameteri(t: u32, p: u32, v: i32) { send(OP_TEX_PARAM_I, &[t, p, v as u32]); }
#[no_mangle] pub unsafe extern "C" fn glTexParameterf(t: u32, p: u32, v: f32) { send(OP_TEX_PARAM_I, &[t, p, v as i32 as u32]); }
#[no_mangle] pub unsafe extern "C" fn glGenerateMipmap(t: u32) { send(OP_GEN_MIPMAP, &[t]); }
const MAX_CHUNK: usize = 1 << 20;          // 1 MiB má»—i báº£n ghi
unsafe fn tex_upload(op: u32, head: &[u32], w: i32, h: i32, fmt: u32, ty: u32, pixels: *const u8, sub: bool) {
    // head Ä‘Ã£ cÃ³ Ä‘á»§ tham sá»‘ trá»« pháº§n dá»¯ liá»‡u; dá»¯ liá»‡u cáº¯t theo hÃ ng náº¿u quÃ¡ lá»›n
    let bpp = tex_bpp(fmt, ty);
    let row = (((w as u32 * bpp) + UNPACK_ALIGN - 1) / UNPACK_ALIGN) * UNPACK_ALIGN;
    let total = row as usize * h as usize;
    if pixels.is_null() { send_blob(op, head, null(), 0); return; }
    if total <= MAX_CHUNK { send_blob(op, head, pixels, total); return; }
    let rows_per = (MAX_CHUNK / row as usize).max(1);
    // cáº¥p phÃ¡t báº±ng TEX_IMAGE (khÃ´ng dá»¯ liá»‡u) rá»“i náº¡p tá»«ng dáº£i báº±ng TEX_SUBIMAGE
    if !sub { let mut hh = [0u32; 16]; let n = head.len(); let mut k = 0; while k < n { hh[k] = head[k]; k += 1; } send_blob(OP_TEX_IMAGE, &hh[..n], null(), 0); }
    let mut y = 0usize;
    while y < h as usize {
        let r = if y + rows_per > h as usize { h as usize - y } else { rows_per };
        // TEX_SUBIMAGE: target level xoff yoff w h fmt type
        let target = head[0]; let level = head[1];
        send_blob(OP_TEX_SUBIMAGE, &[target, level, 0, y as u32, w as u32, r as u32, fmt, ty], pixels.add(y * row as usize), r * row as usize);
        y += r;
    }
}
#[no_mangle] pub unsafe extern "C" fn glTexImage2D(target: u32, level: i32, ifmt: i32, w: i32, h: i32, border: i32, fmt: u32, ty: u32, pixels: *const u8) {
    tex_upload(OP_TEX_IMAGE, &[target, level as u32, ifmt as u32, w as u32, h as u32, border as u32, fmt, ty], w, h, fmt, ty, pixels, false);
}
#[no_mangle] pub unsafe extern "C" fn glTexSubImage2D(target: u32, level: i32, x: i32, y: i32, w: i32, h: i32, fmt: u32, ty: u32, pixels: *const u8) {
    tex_upload(OP_TEX_SUBIMAGE, &[target, level as u32, x as u32, y as u32, w as u32, h as u32, fmt, ty], w, h, fmt, ty, pixels, true);
}
#[no_mangle] pub unsafe extern "C" fn glCopyTexImage2D(t: u32, l: i32, f: u32, x: i32, y: i32, w: i32, h: i32, b: i32) { send(OP_COPY_TEX_IMAGE, &[t, l as u32, f, x as u32, y as u32, w as u32, h as u32, b as u32]); }

// ---- framebuffer object ----
#[no_mangle] pub unsafe extern "C" fn glGenFramebuffers(n: i32, out: *mut u32) { gen_objs(OP_GEN_FRAMEBUFFERS, n, out); }
#[no_mangle] pub unsafe extern "C" fn glBindFramebuffer(t: u32, id: u32) { send(OP_BIND_FRAMEBUFFER, &[t, id]); }
#[no_mangle] pub unsafe extern "C" fn glFramebufferTexture2D(t: u32, a: u32, tt: u32, tex: u32, lvl: i32) { send(OP_FB_TEXTURE2D, &[t, a, tt, tex, lvl as u32]); }
#[no_mangle] pub unsafe extern "C" fn glGenRenderbuffers(n: i32, out: *mut u32) { gen_objs(OP_GEN_RENDERBUFFERS, n, out); }
#[no_mangle] pub unsafe extern "C" fn glBindRenderbuffer(t: u32, id: u32) { send(OP_BIND_RENDERBUFFER, &[t, id]); }
#[no_mangle] pub unsafe extern "C" fn glRenderbufferStorage(t: u32, f: u32, w: i32, h: i32) { send(OP_RB_STORAGE, &[t, f, w as u32, h as u32]); }
#[no_mangle] pub unsafe extern "C" fn glFramebufferRenderbuffer(t: u32, a: u32, rt: u32, rb: u32) { send(OP_FB_RENDERBUFFER, &[t, a, rt, rb]); }
#[no_mangle] pub unsafe extern "C" fn glCheckFramebufferStatus(t: u32) -> u32 { let r = call(OP_CHECK_FB_STATUS, &[t]); if r.is_null() { 0x8CD5 } else { read_volatile(r) } }
#[no_mangle] pub unsafe extern "C" fn glDeleteFramebuffers(n: i32, ids: *const u32) { let mut i = 0; while i < n as usize { send(OP_DELETE_OBJS, &[4, 1, *ids.add(i)]); i += 1; } }
#[no_mangle] pub unsafe extern "C" fn glDeleteRenderbuffers(n: i32, ids: *const u32) { let mut i = 0; while i < n as usize { send(OP_DELETE_OBJS, &[5, 1, *ids.add(i)]); i += 1; } }
#[no_mangle] pub unsafe extern "C" fn glReadPixels(x: i32, y: i32, w: i32, h: i32, fmt: u32, ty: u32, out: *mut u8) {
    let bpp = tex_bpp(fmt, ty); let row = (((w as u32 * bpp) + PACK_ALIGN - 1) / PACK_ALIGN) * PACK_ALIGN;
    let total = row as usize * h as usize; if out.is_null() || total > RESP_WORDS * 4 - 8 { return; }
    let r = call(OP_READ_PIXELS, &[x as u32, y as u32, w as u32, h as u32, fmt, ty, row]);
    if !r.is_null() { memcpy(out, r as *const u8, total); }
}

// =====================================================================================
// eglGetProcAddress: tra báº£ng tÃªn -> hÃ m
// =====================================================================================
unsafe fn eq(name: *const u8, lit: &[u8]) -> bool {
    let mut i = 0; while i < lit.len() { if read_volatile(name.add(i)) != lit[i] { return false; } i += 1; } read_volatile(name.add(lit.len())) == 0
}
macro_rules! procs { ($name:expr, $($f:ident),* $(,)?) => { $( if eq($name, stringify!($f).as_bytes()) { return $f as *const () as usize; } )* }; }
#[no_mangle] pub unsafe extern "C" fn eglGetProcAddress(name: *const u8) -> usize {
    if name.is_null() { return 0; }
    trace(b"eglGetProcAddress", name);
    procs!(name, eglGetDisplay, eglInitialize, eglTerminate, eglGetError, eglBindAPI, eglChooseConfig, eglGetConfigAttrib,
        eglCreateWindowSurface, eglCreatePbufferSurface, eglDestroySurface, eglCreateContext, eglDestroyContext, eglMakeCurrent,
        eglSwapInterval, eglSwapBuffers, eglQuerySurface, eglQueryString, eglGetProcAddress, eglGetCurrentContext, eglGetCurrentSurface,
        eglGetCurrentDisplay, eglReleaseThread, eglWaitGL, eglWaitNative, eglQueryContext, eglGetConfigs, eglSurfaceAttrib);
    procs!(name, glShaderBinary, glReleaseShaderCompiler, glDetachShader, glIsTexture, glClearColor, glClear, glClearDepthf, glClearStencil, glViewport, glScissor, glEnable, glDisable, glBlendFunc,
        glBlendFuncSeparate, glBlendEquation, glBlendEquationSeparate, glBlendColor, glDepthFunc, glDepthMask, glDepthRangef, glColorMask,
        glFrontFace, glCullFace, glLineWidth, glHint, glStencilFunc, glStencilOp, glStencilMask, glPixelStorei, glFinish, glFlush,
        glGetError, glActiveTexture, glGetString, glGetIntegerv, glGetBooleanv, glGetFloatv, glGetShaderPrecisionFormat, glIsEnabled,
        glCreateShader, glShaderSource, glCompileShader, glGetShaderiv, glGetShaderInfoLog, glGetProgramInfoLog, glCreateProgram,
        glAttachShader, glLinkProgram, glGetProgramiv, glUseProgram, glBindAttribLocation, glGetAttribLocation, glGetUniformLocation,
        glDeleteShader, glDeleteProgram, glValidateProgram);
    procs!(name, glUniform1f, glUniform2f, glUniform3f, glUniform4f, glUniform1i, glUniform2i, glUniform3i, glUniform4i, glUniform1fv,
        glUniform2fv, glUniform3fv, glUniform4fv, glUniformMatrix2fv, glUniformMatrix3fv, glUniformMatrix4fv, glEnableVertexAttribArray,
        glDisableVertexAttribArray, glVertexAttribPointer, glVertexAttrib1f, glVertexAttrib2f, glVertexAttrib3f, glVertexAttrib4f,
        glGenBuffers, glBindBuffer, glBufferData, glBufferSubData, glDeleteBuffers, glDrawArrays, glDrawElements, glGenTextures,
        glBindTexture, glDeleteTextures, glTexParameteri, glTexParameterf, glGenerateMipmap, glTexImage2D, glTexSubImage2D,
        glCopyTexImage2D, glGenFramebuffers, glBindFramebuffer, glFramebufferTexture2D, glGenRenderbuffers, glBindRenderbuffer,
        glRenderbufferStorage, glFramebufferRenderbuffer, glCheckFramebufferStatus, glDeleteFramebuffers, glDeleteRenderbuffers, glReadPixels);
    0
}





// Go loi: neu ton tai /tmp/glremote.debug thi ghi ten ham EGL vao /tmp/glremote.trace
unsafe fn trace(name: &[u8], arg: *const u8) {
    let dbg = b"/tmp/glremote.debug\0";
    let f = sc6(SYS_OPEN, dbg.as_ptr() as u32, 0, 0, 0, 0, 0);
    if f < 0 { return; }
    sc6(SYS_CLOSE, f as u32, 0, 0, 0, 0, 0);
    let p = b"/tmp/glremote.trace\0";
    let fd = sc6(SYS_OPEN, p.as_ptr() as u32, 0x441, 0o666, 0, 0, 0);
    if fd < 0 { return; }
    sc6(4, fd as u32, name.as_ptr() as u32, name.len() as u32, 0, 0, 0);
    if !arg.is_null() { sc6(4, fd as u32, b" ".as_ptr() as u32, 1, 0, 0, 0); sc6(4, fd as u32, arg as u32, cstrlen(arg) as u32, 0, 0, 0); }
    sc6(4, fd as u32, b"\n".as_ptr() as u32, 1, 0, 0, 0);
    sc6(SYS_CLOSE, fd as u32, 0, 0, 0, 0, 0);
}
