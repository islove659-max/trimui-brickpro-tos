#![no_std]
#![no_main]
//! Thu duong SDL_Renderer (nhu Apotris): CreateWindow khong co co OPENGL -> CreateRenderer -> RenderClear/Present.
use core::arch::asm;
use core::panic::PanicInfo;
#[panic_handler] fn panic(_: &PanicInfo) -> ! { loop {} }
#[no_mangle] pub extern "C" fn __aeabi_unwind_cpp_pr0() {}
#[no_mangle] pub extern "C" fn __aeabi_unwind_cpp_pr1() {}
#[link(name = "SDL2-2.0")]
extern "C" {
    fn SDL_Init(f: u32) -> i32; fn SDL_GetError() -> *const u8; fn SDL_Quit();
    fn SDL_CreateWindow(t: *const u8, x: i32, y: i32, w: i32, h: i32, f: u32) -> usize;
    fn SDL_CreateRenderer(w: usize, i: i32, f: u32) -> usize; fn SDL_RenderClear(r: usize) -> i32; fn SDL_RenderPresent(r: usize);
    fn SDL_SetRenderDrawColor(r: usize, a: u8, b: u8, c: u8, d: u8) -> i32; fn SDL_Delay(ms: u32);
    fn SDL_GetCurrentDisplayMode(i: i32, m: *mut u32) -> i32; fn SDL_GetCurrentVideoDriver() -> *const u8;
}
unsafe fn sys(n: u32, a: u32, b: u32, c: u32) -> i32 { let r: i32; asm!("svc 0", in("r7") n, inlateout("r0") a => r, in("r1") b, in("r2") c, options(nostack)); r }
unsafe fn out(s: &[u8]) { sys(4, 1, s.as_ptr() as u32, s.len() as u32); }
unsafe fn out_num(mut v: u32) { let mut b = [0u8; 12]; let mut i = 12; if v == 0 { i -= 1; b[i] = b'0'; } while v > 0 { i -= 1; b[i] = b'0' + (v % 10) as u8; v /= 10; } out(&b[i..]); }
unsafe fn cstr(p: *const u8) { if p.is_null() { out(b"(null)"); return; } let mut n = 0; while core::ptr::read_volatile(p.add(n)) != 0 { n += 1; } out(core::slice::from_raw_parts(p, n)); }
unsafe fn fail(m: &[u8]) -> ! { out(m); out(b": "); cstr(SDL_GetError()); out(b"\n"); sys(1, 1, 0, 0); loop {} }
#[no_mangle]
pub unsafe extern "C" fn _start() -> ! {
    if SDL_Init(0x20 | 0x10 | 0x200) != 0 { fail(b"SDL_Init"); }
    out(b"driver: "); cstr(SDL_GetCurrentVideoDriver()); out(b"\n");
    let mut m = [0u32; 6];
    SDL_GetCurrentDisplayMode(0, m.as_mut_ptr()); out(b"mode "); out_num(m[1]); out(b"x"); out_num(m[2]); out(b"\n");
    out(b"CreateWindow...\n");
    let win = SDL_CreateWindow(b"rtest\0".as_ptr(), 0, 0, 1024, 768, 0);   // khong co co OPENGL
    if win == 0 { fail(b"CreateWindow"); }
    out(b"CreateRenderer...\n");
    let r = SDL_CreateRenderer(win, -1, 0);
    if r == 0 { fail(b"CreateRenderer"); }
    out(b"OK, ve 3 giay\n");
    let mut i = 0u32;
    while i < 180 { SDL_SetRenderDrawColor(r, (i * 2) as u8, 60, 160, 255); SDL_RenderClear(r); SDL_RenderPresent(r); i += 1; }
    out(b"xong\n"); SDL_Quit(); sys(1, 0, 0, 0); loop {}
}
#[no_mangle] pub unsafe extern "C" fn memset(d: *mut u8, c: i32, n: usize) -> *mut u8 { let mut i = 0; while i < n { core::ptr::write_volatile(d.add(i), c as u8); i += 1; } d }
#[no_mangle] pub unsafe extern "C" fn memcpy(d: *mut u8, s: *const u8, n: usize) -> *mut u8 { let mut i = 0; while i < n { core::ptr::write_volatile(d.add(i), core::ptr::read_volatile(s.add(i))); i += 1; } d }
