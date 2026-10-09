//! libasound.so.2 GIA cho SDL2 (driver alsa): chep PCM S16 vao bo nho chung /tmp/glremote.snd,
//! tien trinh `glserver --audio` (64-bit) doc ra /dev/dsp. Chi cai dat tap ham ma SDL2 2.0.14 dung.
#![no_std]
#![allow(non_snake_case, static_mut_refs)]
use core::arch::asm;
use core::panic::PanicInfo;
use core::ptr::{read_volatile, write_volatile};
use core::sync::atomic::{fence, Ordering};
#[panic_handler] fn panic(_: &PanicInfo) -> ! { loop {} }
#[no_mangle] pub extern "C" fn __aeabi_unwind_cpp_pr0() {}
#[no_mangle] pub extern "C" fn __aeabi_unwind_cpp_pr1() {}
#[no_mangle] pub unsafe extern "C" fn memcpy(d: *mut u8, s: *const u8, n: usize) -> *mut u8 { let mut i = 0; while i < n { write_volatile(d.add(i), read_volatile(s.add(i))); i += 1; } d }
#[no_mangle] pub unsafe extern "C" fn memset(d: *mut u8, c: i32, n: usize) -> *mut u8 { let mut i = 0; while i < n { write_volatile(d.add(i), c as u8); i += 1; } d }

#[inline(always)]
unsafe fn sc6(n: u32, a: u32, b: u32, c: u32, d: u32, e: u32, f: u32) -> i32 {
    let r: i32;
    asm!("svc 0", in("r7") n, inlateout("r0") a => r, in("r1") b, in("r2") c, in("r3") d, in("r4") e, in("r5") f, options(nostack));
    r
}
unsafe fn sleep_us(us: u32) { let ts = [0u32, us * 1000]; sc6(162, ts.as_ptr() as u32, 0, 0, 0, 0, 0); }

// Bo nho chung am thanh (KHOP glserver --audio): 64 byte tieu de + vong 256 KiB
const RING: usize = 262144; const FILE: usize = 64 + RING;
const H_MAGIC: usize = 0; const H_RATE: usize = 1; const H_CH: usize = 2; const H_ACTIVE: usize = 3; const H_WPOS: usize = 4; const H_RPOS: usize = 5; const H_GEN: usize = 6; const H_READY: usize = 7;
static mut SND: *mut u32 = core::ptr::null_mut();
unsafe fn connect() -> bool {
    if !SND.is_null() { return true; }
    let path = b"/tmp/glremote.snd\0";
    let fd = sc6(5, path.as_ptr() as u32, 2, 0, 0, 0, 0);
    if fd < 0 { return false; }
    let p = sc6(192, 0, FILE as u32, 3, 1, fd as u32, 0);
    sc6(6, fd as u32, 0, 0, 0, 0, 0);
    if (p as u32) > 0xFFFF_F000 { return false; }
    SND = p as *mut u32;
    read_volatile(SND.add(H_READY)) != 0
}
#[inline(always)] unsafe fn h(i: usize) -> *mut u32 { SND.add(i) }

// hw_params: [rate, channels, period_frames, buffer_frames]
unsafe fn hp(p: *mut u32, i: usize) -> *mut u32 { p.add(i) }
static mut CH: u32 = 2;
static mut OPEN: bool = false;

#[no_mangle] pub unsafe extern "C" fn snd_pcm_open(pcm: *mut usize, _name: *const u8, _stream: i32, _mode: i32) -> i32 {
    if !connect() { return -2; }
    *pcm = 0x5A5A0001; OPEN = true; 0
}
#[no_mangle] pub unsafe extern "C" fn snd_pcm_close(_p: usize) -> i32 { if !SND.is_null() { write_volatile(h(H_ACTIVE), 0); } OPEN = false; 0 }
#[no_mangle] pub extern "C" fn snd_pcm_hw_params_sizeof() -> usize { 256 }
#[no_mangle] pub extern "C" fn snd_pcm_sw_params_sizeof() -> usize { 256 }
#[no_mangle] pub unsafe extern "C" fn snd_pcm_hw_params_any(_p: usize, params: *mut u32) -> i32 { *hp(params, 0) = 48000; *hp(params, 1) = 2; *hp(params, 2) = 1024; *hp(params, 3) = 4096; 0 }
#[no_mangle] pub unsafe extern "C" fn snd_pcm_hw_params_copy(dst: *mut u32, src: *const u32) { memcpy(dst as *mut u8, src as *const u8, 256); }
#[no_mangle] pub extern "C" fn snd_pcm_hw_params_set_access(_p: usize, _h: *mut u32, _a: u32) -> i32 { 0 }
#[no_mangle] pub extern "C" fn snd_pcm_hw_params_set_format(_p: usize, _h: *mut u32, f: i32) -> i32 { if f == 2 { 0 } else { -22 } }   // chi S16_LE
#[no_mangle] pub unsafe extern "C" fn snd_pcm_hw_params_set_channels(_p: usize, params: *mut u32, c: u32) -> i32 { if c == 1 || c == 2 { *hp(params, 1) = c; 0 } else { -22 } }
#[no_mangle] pub unsafe extern "C" fn snd_pcm_hw_params_get_channels(params: *const u32, out: *mut u32) -> i32 { *out = *params.add(1); 0 }
#[no_mangle] pub unsafe extern "C" fn snd_pcm_hw_params_set_rate_near(_p: usize, params: *mut u32, val: *mut u32, _dir: *mut i32) -> i32 { *val = 48000; *hp(params, 0) = 48000; 0 }
#[no_mangle] pub unsafe extern "C" fn snd_pcm_hw_params_set_period_size_near(_p: usize, params: *mut u32, val: *mut u32, _dir: *mut i32) -> i32 { if *val < 256 { *val = 256; } *hp(params, 2) = *val; *hp(params, 3) = *val * 4; 0 }
#[no_mangle] pub unsafe extern "C" fn snd_pcm_hw_params_get_buffer_size(params: *const u32, out: *mut u32) -> i32 { *out = *params.add(3); 0 }
#[no_mangle] pub extern "C" fn snd_pcm_hw_params_set_periods_min(_p: usize, _h: *mut u32, _v: *mut u32, _d: *mut i32) -> i32 { 0 }
#[no_mangle] pub extern "C" fn snd_pcm_hw_params_set_periods_first(_p: usize, _h: *mut u32, _v: *mut u32, _d: *mut i32) -> i32 { 0 }
#[no_mangle] pub unsafe extern "C" fn snd_pcm_hw_params(_p: usize, params: *const u32) -> i32 {
    if SND.is_null() { return -2; }
    CH = *params.add(1);
    write_volatile(h(H_RATE), *params); write_volatile(h(H_CH), CH);
    write_volatile(h(H_RPOS), read_volatile(h(H_WPOS)));
    fence(Ordering::Release);
    write_volatile(h(H_GEN), read_volatile(h(H_GEN)).wrapping_add(1));
    write_volatile(h(H_ACTIVE), 1);
    0
}
#[no_mangle] pub extern "C" fn snd_pcm_sw_params_current(_p: usize, _s: *mut u32) -> i32 { 0 }
#[no_mangle] pub extern "C" fn snd_pcm_sw_params_set_avail_min(_p: usize, _s: *mut u32, _v: u32) -> i32 { 0 }
#[no_mangle] pub extern "C" fn snd_pcm_sw_params_set_start_threshold(_p: usize, _s: *mut u32, _v: u32) -> i32 { 0 }
#[no_mangle] pub extern "C" fn snd_pcm_sw_params(_p: usize, _s: *mut u32) -> i32 { 0 }
#[no_mangle] pub extern "C" fn snd_pcm_nonblock(_p: usize, _n: i32) -> i32 { 0 }
#[no_mangle] pub extern "C" fn snd_pcm_recover(_p: usize, _e: i32, _s: i32) -> i32 { 0 }
#[no_mangle] pub extern "C" fn snd_pcm_wait(_p: usize, _t: i32) -> i32 { 1 }
#[no_mangle] pub extern "C" fn snd_pcm_reset(_p: usize) -> i32 { 0 }
#[no_mangle] pub extern "C" fn snd_pcm_readi(_p: usize, _b: *mut u8, _f: u32) -> i32 { -22 }
#[no_mangle] pub extern "C" fn snd_pcm_get_chmap(_p: usize) -> usize { 0 }
#[no_mangle] pub extern "C" fn snd_pcm_chmap_print(_m: usize, _n: u32, _b: *mut u8) -> i32 { 0 }
#[no_mangle] pub extern "C" fn snd_strerror(_e: i32) -> *const u8 { b"loi am thanh\0".as_ptr() }
static mut HINTS: [usize; 2] = [0; 2];
#[no_mangle] pub unsafe extern "C" fn snd_device_name_hint(_c: i32, _i: *const u8, hints: *mut usize) -> i32 { *hints = HINTS.as_mut_ptr() as usize; 0 }
#[no_mangle] pub extern "C" fn snd_device_name_get_hint(_h: usize, _id: *const u8) -> usize { 0 }
#[no_mangle] pub extern "C" fn snd_device_name_free_hint(_h: usize) -> i32 { 0 }

/// Ghi `frames` khung S16 vao vong; chan khi day de khop toc do phat that.
#[no_mangle] pub unsafe extern "C" fn snd_pcm_writei(_p: usize, buf: *const u8, frames: u32) -> i32 {
    if SND.is_null() { return -32; }
    let total = (frames * CH * 2) as usize; let mut done = 0usize; let ring = (SND as *mut u8).add(64);
    let mut idle = 0u32;
    while done < total {
        let w = read_volatile(h(H_WPOS)); let r = read_volatile(h(H_RPOS));
        let free = RING - (w.wrapping_sub(r) as usize);
        if free == 0 { sleep_us(1000); idle += 1; if idle > 3000 { return -32; } continue; }   // 3 giay khong ai doc => loi EPIPE
        idle = 0;
        let off = (w as usize) % RING; let n = core::cmp::min(core::cmp::min(free, total - done), RING - off);
        memcpy(ring.add(off), buf.add(done), n);
        fence(Ordering::Release);
        write_volatile(h(H_WPOS), w.wrapping_add(n as u32));
        done += n;
    }
    frames as i32
}
