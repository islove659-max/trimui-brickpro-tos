#![no_std]
#![no_main]
//! Ung dung thu 32-bit dung SDL2 that (Debian armhf) -> EGL/GLES cua glremote. Chay voi SDL_VIDEODRIVER=offscreen.
use core::arch::asm;
use core::panic::PanicInfo;
#[panic_handler]
fn panic(_: &PanicInfo) -> ! { loop {} }
#[no_mangle] pub extern "C" fn __aeabi_unwind_cpp_pr0() {}
#[no_mangle] pub extern "C" fn __aeabi_unwind_cpp_pr1() {}

#[link(name = "SDL2-2.0")]
extern "C" {
    fn SDL_Init(f: u32) -> i32; fn SDL_GetError() -> *const u8; fn SDL_GL_SetAttribute(a: i32, v: i32) -> i32;
    fn SDL_CreateWindow(t: *const u8, x: i32, y: i32, w: i32, h: i32, f: u32) -> usize; fn SDL_GL_CreateContext(w: usize) -> usize;
    fn SDL_GL_SwapWindow(w: usize); fn SDL_GL_GetProcAddress(n: *const u8) -> usize; fn SDL_GL_SetSwapInterval(i: i32) -> i32;
    fn SDL_GetCurrentVideoDriver() -> *const u8; fn SDL_Quit(); fn SDL_InitSubSystem(f: u32) -> i32; fn SDL_OpenAudioDevice(n: *const u8, c: i32, d: *const u32, o: *mut u32, a: i32) -> u32; fn SDL_PauseAudioDevice(d: u32, p: i32); fn SDL_DestroyWindow(w: usize); fn SDL_QuitSubSystem(f: u32); fn SDL_CloseAudioDevice(d: u32); fn SDL_GL_DeleteContext(c: usize); fn SDL_PumpEvents();
}
macro_rules! glfn { ($($n:ident : fn($($a:ty),*) $(-> $r:ty)?),*) => {
    $( static mut $n: Option<unsafe extern "C" fn($($a),*) $(-> $r)?> = None; )*
    unsafe fn load_all() { $( $n = Some(core::mem::transmute::<usize, unsafe extern "C" fn($($a),*) $(-> $r)?>(SDL_GL_GetProcAddress(concat!(stringify!($n), "\0").as_ptr()))); )* } } }
glfn! {
    glClearColor: fn(f32, f32, f32, f32), glClear: fn(u32), glViewport: fn(i32, i32, i32, i32),
    glCreateShader: fn(u32) -> u32, glShaderSource: fn(u32, i32, *const *const u8, *const i32), glCompileShader: fn(u32),
    glGetShaderiv: fn(u32, u32, *mut i32), glCreateProgram: fn() -> u32, glAttachShader: fn(u32, u32), glLinkProgram: fn(u32), glUseProgram: fn(u32),
    glGetAttribLocation: fn(u32, *const u8) -> i32, glGetUniformLocation: fn(u32, *const u8) -> i32, glUniform1f: fn(i32, f32), glUniform1i: fn(i32, i32),
    glEnableVertexAttribArray: fn(u32), glVertexAttribPointer: fn(u32, i32, u32, u8, i32, usize), glDrawArrays: fn(u32, i32, i32),
    glGenTextures: fn(i32, *mut u32), glBindTexture: fn(u32, u32), glTexImage2D: fn(u32, i32, i32, i32, i32, i32, u32, u32, *const u8),
    glTexParameteri: fn(u32, u32, i32), glGetString: fn(u32) -> *const u8
}
macro_rules! gl { ($n:ident($($a:expr),*)) => { ($n.unwrap())($($a),*) } }

unsafe fn sys(n: u32, a: u32, b: u32, c: u32) -> i32 { let r: i32; asm!("svc 0", in("r7") n, inlateout("r0") a => r, in("r1") b, in("r2") c, options(nostack)); r }
unsafe fn sys6(n: u32, a: u32, b: u32, c: u32, d: u32) -> i32 { let r: i32; asm!("svc 0", in("r7") n, inlateout("r0") a => r, in("r1") b, in("r2") c, in("r3") d, options(nostack)); r }
unsafe fn out(s: &[u8]) { sys(4, 1, s.as_ptr() as u32, s.len() as u32); }
unsafe fn out_num(mut v: u32) { let mut b = [0u8; 12]; let mut i = 12; if v == 0 { i -= 1; b[i] = b'0'; } while v > 0 { i -= 1; b[i] = b'0' + (v % 10) as u8; v /= 10; } out(&b[i..]); }
unsafe fn cstr(p: *const u8) { if p.is_null() { out(b"(null)"); return; } let mut n = 0; while core::ptr::read_volatile(p.add(n)) != 0 { n += 1; } out(core::slice::from_raw_parts(p, n)); }
unsafe fn now_ms() -> u32 { let mut ts = [0u32; 2]; sys(263, 1, ts.as_mut_ptr() as u32, 0); ts[0] * 1000 + ts[1] / 1_000_000 }
unsafe fn fail(m: &[u8]) -> ! { out(m); out(b": "); cstr(SDL_GetError()); out(b"\n"); sys(1, 1, 0, 0); loop {} }

static mut AUDIO_CB: u32 = 0; static mut PHASE: u32 = 0;
unsafe extern "C" fn audio_cb(_u: usize, stream: *mut u8, len: i32) {
    AUDIO_CB += 1; let n = (len / 2) as usize; let s = stream as *mut i16; let mut i = 0;
    while i < n { PHASE = PHASE.wrapping_add(1); let v: i16 = if (PHASE / 55) & 1 == 0 { 3000 } else { -3000 }; core::ptr::write_volatile(s.add(i), v); i += 1; }
}
unsafe fn hex(v: u32) { let mut b = [0u8; 10]; b[0] = b'0'; b[1] = b'x'; let mut i = 0; while i < 8 { let n = ((v >> (28 - 4 * i)) & 15) as u8; b[2 + i] = if n < 10 { b'0' + n } else { b'a' + n - 10 }; i += 1; } out(&b); }
unsafe extern "C" fn on_segv(_s: i32, info: *const u32, uc: *const u32) {
    out(b"SEGV addr="); hex(*info.add(3)); out(b" pc="); hex(*uc.add(23)); out(b" lr="); hex(*uc.add(22)); out(b"\n");
    let pc = *uc.add(23); let lr = *uc.add(22);
    let fd = sys(5, b"/proc/self/maps\0".as_ptr() as u32, 0, 0); static mut BUF: [u8; 65536] = [0; 65536];
    let mut len = 0usize;
    loop { let n = sys(3, fd as u32, BUF.as_mut_ptr().add(len) as u32, (65536 - len) as u32); if n <= 0 { break; } len += n as usize; if len >= 65536 { break; } }
    let mut ls = 0usize; let mut i = 0usize;
    while i <= len {
        if i == len || BUF[i] == b'\n' {
            // dong [ls, i): "start-end perms off dev inode path"
            let mut s = 0u32; let mut e = 0u32; let mut k = ls; let mut dash = false;
            while k < i && BUF[k] != b' ' { let c = BUF[k]; if c == b'-' { dash = true; } else { let v = if c <= b'9' { c - b'0' } else { c - b'a' + 10 } as u32; if dash { e = (e << 4) | v; } else { s = (s << 4) | v; } } k += 1; }
            if (pc >= s && pc < e) || (lr >= s && lr < e) { out(b"MAP "); out(&BUF[ls..i]); out(b" (off pc="); hex(pc.wrapping_sub(s)); out(b")\n"); }
            ls = i + 1;
        }
        i += 1;
    }    sys(1, 139, 0, 0);
}
static VS: &[u8] = b"attribute vec2 p; attribute vec2 t; uniform float a; varying vec2 v; void main(){ float c=cos(a), s=sin(a); gl_Position=vec4(p.x*c-p.y*s, p.x*s+p.y*c, 0.0, 1.0); v=t; }";
static FS: &[u8] = b"precision mediump float; varying vec2 v; uniform sampler2D tx; void main(){ gl_FragColor = texture2D(tx, v) * vec4(v, 1.0, 1.0); }";
unsafe fn shader(kind: u32, src: &[u8]) -> u32 {
    let s = gl!(glCreateShader(kind)); let p = src.as_ptr(); let l = src.len() as i32;
    gl!(glShaderSource(s, 1, &p, &l)); gl!(glCompileShader(s));
    let mut ok = 0; gl!(glGetShaderiv(s, 0x8B81, &mut ok)); if ok == 0 { out(b"shader LOI\n"); sys(1, 1, 0, 0); } s
}

#[no_mangle]
pub unsafe extern "C" fn _start() -> ! {
    { let act: [u32; 5] = [on_segv as usize as u32, 4, 0, 0, 0]; sys6(174, 11, act.as_ptr() as u32, 0, 8); }
    if SDL_Init(0x20) != 0 { fail(b"SDL_Init"); }
    out(b"SDL video driver: "); cstr(SDL_GetCurrentVideoDriver()); out(b"\n");
    for (n, fl) in [(&b"audio"[..], 0x10u32), (&b"timer"[..], 1), (&b"joystick"[..], 0x200), (&b"haptic"[..], 0x1000), (&b"gamecontroller"[..], 0x2000)] { out(b"InitSubSystem "); out(n); out(b"...\n"); let r = SDL_InitSubSystem(fl); out(b"  -> "); out_num(r as u32); out(b"\n"); }
    SDL_GL_SetAttribute(17, 2); SDL_GL_SetAttribute(18, 0); SDL_GL_SetAttribute(21, 4);
    let win = SDL_CreateWindow(b"sdl_test\0".as_ptr(), 0, 0, 1024, 768, 0x2 | 0x4);
    if win == 0 { fail(b"SDL_CreateWindow"); }
    if SDL_GL_CreateContext(win) == 0 { fail(b"SDL_GL_CreateContext"); }
    SDL_GL_SetSwapInterval(1);
    load_all();
    out(b"GL_RENDERER: "); cstr(gl!(glGetString(0x1F01))); out(b"\n");
    let vs = shader(0x8B31, VS); let fs = shader(0x8B30, FS);
    let pr = gl!(glCreateProgram()); gl!(glAttachShader(pr, vs)); gl!(glAttachShader(pr, fs)); gl!(glLinkProgram(pr)); gl!(glUseProgram(pr));
    let lp = gl!(glGetAttribLocation(pr, b"p\0".as_ptr())); let lt = gl!(glGetAttribLocation(pr, b"t\0".as_ptr()));
    let la = gl!(glGetUniformLocation(pr, b"a\0".as_ptr())); let ltx = gl!(glGetUniformLocation(pr, b"tx\0".as_ptr()));
    let mut px = [0u8; 16 * 16 * 4]; let mut y = 0; while y < 16 { let mut x = 0; while x < 16 { let c = if ((x / 4) + (y / 4)) % 2 == 0 { 255 } else { 90 }; let o = (y * 16 + x) * 4; px[o] = c; px[o + 1] = c; px[o + 2] = c; px[o + 3] = 255; x += 1; } y += 1; }
    let mut tex = 0; gl!(glGenTextures(1, &mut tex)); gl!(glBindTexture(0x0DE1, tex));
    gl!(glTexImage2D(0x0DE1, 0, 0x1908, 16, 16, 0, 0x1908, 0x1401, px.as_ptr()));
    gl!(glTexParameteri(0x0DE1, 0x2801, 0x2600)); gl!(glTexParameteri(0x0DE1, 0x2800, 0x2600));
    gl!(glUniform1i(ltx, 0));
    let quad: [f32; 24] = [-0.6,-0.6,0.0,0.0,  0.6,-0.6,1.0,0.0,  0.6,0.6,1.0,1.0,   -0.6,-0.6,0.0,0.0,  0.6,0.6,1.0,1.0,  -0.6,0.6,0.0,1.0];
    gl!(glViewport(0, 0, 1024, 768));
    let spec: [u32; 6] = [48000, 0x8010 | (2 << 16), (1024), 0, audio_cb as usize as u32, 0];
    let mut got = [0u32; 6];
    let dev = SDL_OpenAudioDevice(core::ptr::null(), 0, spec.as_ptr(), got.as_mut_ptr(), 0);
    out(b"SELFTEST audio_dev="); out_num(dev); out(b"\n");
    if dev >= 2 { SDL_PauseAudioDevice(dev, 0); }
    let t0 = now_ms(); let mut frames = 0u32; let mut ang = 0.0f32;
    while frames < 150 {
        gl!(glClearColor(0.05, 0.2, 0.08, 1.0)); gl!(glClear(0x4000));
        gl!(glEnableVertexAttribArray(lp as u32)); gl!(glEnableVertexAttribArray(lt as u32));
        gl!(glVertexAttribPointer(lp as u32, 2, 0x1406, 0, 16, quad.as_ptr() as usize));
        gl!(glVertexAttribPointer(lt as u32, 2, 0x1406, 0, 16, quad.as_ptr().add(2) as usize));
        gl!(glUniform1f(la, ang)); gl!(glDrawArrays(4, 0, 6));
        SDL_GL_SwapWindow(win); SDL_PumpEvents(); frames += 1; ang += 0.02;
    }
    let ms = now_ms() - t0;
    let tw = now_ms(); while now_ms() - tw < 1000 { SDL_PumpEvents(); sys(162, 0, 0, 0); }
    out(b"SELFTEST frames="); out_num(frames); out(b" audio_cb="); out_num(AUDIO_CB); out(b"\n");
    out(b"khung: "); out_num(frames); out(b" / ms: "); out_num(ms); out(b"  => FPS x10: "); out_num(frames * 10000 / ms); out(b"\n");
    out(b"STEP closeaudio\n"); if dev >= 2 { SDL_CloseAudioDevice(dev); }
    out(b"STEP destroywindow\n"); SDL_DestroyWindow(win);
    for (n, fl) in [(&b"audio"[..], 0x10u32), (&b"video"[..], 0x20), (&b"timer"[..], 1), (&b"gamecontroller"[..], 0x2000), (&b"joystick"[..], 0x200)] { out(b"STEP quit "); out(n); out(b"\n"); SDL_QuitSubSystem(fl); }
    out(b"STEP quit\n"); SDL_Quit(); out(b"STEP done\n"); sys(1, 0, 0, 0); loop {}
}
#[no_mangle] pub unsafe extern "C" fn memset(d: *mut u8, c: i32, n: usize) -> *mut u8 { let mut i = 0; while i < n { core::ptr::write_volatile(d.add(i), c as u8); i += 1; } d }
#[no_mangle] pub unsafe extern "C" fn memcpy(d: *mut u8, s: *const u8, n: usize) -> *mut u8 { let mut i = 0; while i < n { core::ptr::write_volatile(d.add(i), core::ptr::read_volatile(s.add(i))); i += 1; } d }


