#![no_std]
#![no_main]
//! Ung dung thu 32-bit: EGL + GLES2 qua glremote. Ve tam giac xoay mau + hinh vuong co texture, do FPS.
use core::arch::asm;
use core::panic::PanicInfo;
#[panic_handler]
fn panic(_: &PanicInfo) -> ! { loop {} }
#[no_mangle] pub extern "C" fn __aeabi_unwind_cpp_pr0() {}
#[no_mangle] pub extern "C" fn __aeabi_unwind_cpp_pr1() {}

#[link(name = "glremote")]
extern "C" {
    fn eglGetDisplay(d: usize) -> usize; fn eglInitialize(d: usize, a: *mut i32, b: *mut i32) -> u32;
    fn eglChooseConfig(d: usize, a: *const i32, c: *mut usize, s: i32, n: *mut i32) -> u32;
    fn eglCreateWindowSurface(d: usize, c: usize, w: usize, a: *const i32) -> usize;
    fn eglCreateContext(d: usize, c: usize, s: usize, a: *const i32) -> usize;
    fn eglMakeCurrent(d: usize, a: usize, b: usize, c: usize) -> u32; fn eglSwapBuffers(d: usize, s: usize) -> u32; fn eglTerminate(d: usize) -> u32;
    fn glClearColor(r: f32, g: f32, b: f32, a: f32); fn glClear(m: u32); fn glViewport(x: i32, y: i32, w: i32, h: i32);
    fn glCreateShader(t: u32) -> u32; fn glShaderSource(s: u32, n: i32, p: *const *const u8, l: *const i32); fn glCompileShader(s: u32);
    fn glGetShaderiv(s: u32, p: u32, o: *mut i32);
    fn glCreateProgram() -> u32; fn glAttachShader(p: u32, s: u32); fn glLinkProgram(p: u32); fn glUseProgram(p: u32);
    fn glGetAttribLocation(p: u32, n: *const u8) -> i32; fn glGetUniformLocation(p: u32, n: *const u8) -> i32;
    fn glUniform1f(l: i32, a: f32); fn glUniform1i(l: i32, a: i32);
    fn glEnableVertexAttribArray(i: u32); fn glVertexAttribPointer(i: u32, s: i32, t: u32, n: u8, st: i32, p: usize);
    fn glDrawArrays(m: u32, f: i32, c: i32);
    fn glGenTextures(n: i32, o: *mut u32); fn glBindTexture(t: u32, i: u32); fn glTexImage2D(t: u32, l: i32, f: i32, w: i32, h: i32, b: i32, fm: u32, ty: u32, p: *const u8);
    fn glTexParameteri(t: u32, p: u32, v: i32); fn glGetString(n: u32) -> *const u8;
    fn glEnable(c: u32); fn glBlendFunc(a: u32, b: u32);
}
unsafe fn sys(n: u32, a: u32, b: u32, c: u32) -> i32 { let r: i32; asm!("svc 0", in("r7") n, inlateout("r0") a => r, in("r1") b, in("r2") c, options(nostack)); r }
unsafe fn out(s: &[u8]) { sys(4, 1, s.as_ptr() as u32, s.len() as u32); }
unsafe fn out_num(mut v: u32) { let mut b = [0u8; 12]; let mut i = 12; if v == 0 { i -= 1; b[i] = b'0'; } while v > 0 { i -= 1; b[i] = b'0' + (v % 10) as u8; v /= 10; } out(&b[i..]); }
unsafe fn cstr(p: *const u8) { let mut n = 0; while core::ptr::read_volatile(p.add(n)) != 0 { n += 1; } out(core::slice::from_raw_parts(p, n)); }
unsafe fn now_ms() -> u32 { let mut ts = [0u32; 2]; sys(263, 1, ts.as_mut_ptr() as u32, 0); ts[0] * 1000 + ts[1] / 1_000_000 }

static VS: &[u8] = b"attribute vec2 p; attribute vec2 t; uniform float a; varying vec2 v; void main(){ float c=cos(a), s=sin(a); gl_Position=vec4(p.x*c-p.y*s, p.x*s+p.y*c, 0.0, 1.0); v=t; }";
static FS: &[u8] = b"precision mediump float; varying vec2 v; uniform sampler2D tx; void main(){ gl_FragColor = texture2D(tx, v) * vec4(v, 1.0, 1.0); }";

unsafe fn shader(kind: u32, src: &[u8]) -> u32 {
    let s = glCreateShader(kind); let p = src.as_ptr(); let l = src.len() as i32;
    glShaderSource(s, 1, &p, &l); glCompileShader(s);
    let mut ok = 0; glGetShaderiv(s, 0x8B81, &mut ok); if ok == 0 { out(b"shader LOI\n"); sys(1, 1, 0, 0); } s
}

#[no_mangle]
pub unsafe extern "C" fn _start() -> ! {
    let d = eglGetDisplay(0); let (mut ma, mut mi) = (0, 0);
    if eglInitialize(d, &mut ma, &mut mi) == 0 { out(b"eglInitialize LOI (may chu gl_server chua chay?)\n"); sys(1, 2, 0, 0); }
    let mut cfg = 0usize; let mut nc = 0; eglChooseConfig(d, core::ptr::null(), &mut cfg, 1, &mut nc);
    let surf = eglCreateWindowSurface(d, cfg, 0, core::ptr::null()); let ctx = eglCreateContext(d, cfg, 0, core::ptr::null());
    eglMakeCurrent(d, surf, surf, ctx);
    out(b"GL_RENDERER: "); cstr(glGetString(0x1F01)); out(b"\n");
    let vs = shader(0x8B31, VS); let fs = shader(0x8B30, FS);
    let pr = glCreateProgram(); glAttachShader(pr, vs); glAttachShader(pr, fs); glLinkProgram(pr); glUseProgram(pr);
    let lp = glGetAttribLocation(pr, b"p\0".as_ptr()); let lt = glGetAttribLocation(pr, b"t\0".as_ptr());
    let la = glGetUniformLocation(pr, b"a\0".as_ptr()); let ltx = glGetUniformLocation(pr, b"tx\0".as_ptr());
    // texture 16x16 ban co
    let mut px = [0u8; 16 * 16 * 4]; let mut y = 0; while y < 16 { let mut x = 0; while x < 16 { let c = if ((x / 4) + (y / 4)) % 2 == 0 { 255 } else { 90 }; let o = (y * 16 + x) * 4; px[o] = c; px[o + 1] = c; px[o + 2] = c; px[o + 3] = 255; x += 1; } y += 1; }
    let mut tex = 0; glGenTextures(1, &mut tex); glBindTexture(0x0DE1, tex);
    glTexImage2D(0x0DE1, 0, 0x1908, 16, 16, 0, 0x1908, 0x1401, px.as_ptr());
    glTexParameteri(0x0DE1, 0x2801, 0x2600); glTexParameteri(0x0DE1, 0x2800, 0x2600);
    glUniform1i(ltx, 0);
    // dinh: x,y,u,v — mang trong bo nho client (khong VBO) de thu duong upload client-array
    let quad: [f32; 24] = [-0.6,-0.6,0.0,0.0,  0.6,-0.6,1.0,0.0,  0.6,0.6,1.0,1.0,   -0.6,-0.6,0.0,0.0,  0.6,0.6,1.0,1.0,  -0.6,0.6,0.0,1.0];
    glViewport(0, 0, 1024, 768);
    let t0 = now_ms(); let mut frames = 0u32; let mut ang = 0.0f32;
    while now_ms() - t0 < 6000 {
        glClearColor(0.05, 0.08, 0.2, 1.0); glClear(0x4000);
        glEnableVertexAttribArray(lp as u32); glEnableVertexAttribArray(lt as u32);
        glVertexAttribPointer(lp as u32, 2, 0x1406, 0, 16, quad.as_ptr() as usize);
        glVertexAttribPointer(lt as u32, 2, 0x1406, 0, 16, quad.as_ptr().add(2) as usize);
        glUniform1f(la, ang); glDrawArrays(4, 0, 6);
        eglSwapBuffers(d, surf); frames += 1; ang += 0.02;
    }
    let ms = now_ms() - t0;
    out(b"khung: "); out_num(frames); out(b" / ms: "); out_num(ms); out(b"  => FPS x10: "); out_num(frames * 10000 / ms); out(b"\n");
    eglTerminate(d); sys(1, 0, 0, 0); loop {}
}

