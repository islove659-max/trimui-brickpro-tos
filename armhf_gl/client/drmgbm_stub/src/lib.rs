//! libdrm + libgbm GIA cho SDL2 KMSDRM (driver KMSDRM_LEGACY) tren Brick Pro: mot man hinh 1024x768 ao.
//! Khung hinh that duoc ve va dua len man hinh boi glserver (qua glremote); o day chi tra du lieu hop le.
#![no_std]
#![allow(non_snake_case, non_upper_case_globals, static_mut_refs)]
use core::panic::PanicInfo;
use core::ptr::{null_mut, read_volatile};
#[panic_handler] fn panic(_: &PanicInfo) -> ! { loop {} }
#[no_mangle] pub extern "C" fn __aeabi_unwind_cpp_pr0() {}
#[no_mangle] pub extern "C" fn __aeabi_unwind_cpp_pr1() {}
#[no_mangle] pub unsafe extern "C" fn memset(d: *mut u8, c: i32, n: usize) -> *mut u8 { let mut i = 0; while i < n { core::ptr::write_volatile(d.add(i), c as u8); i += 1; } d }
#[no_mangle] pub unsafe extern "C" fn memcpy(d: *mut u8, s: *const u8, n: usize) -> *mut u8 { let mut i = 0; while i < n { core::ptr::write_volatile(d.add(i), read_volatile(s.add(i))); i += 1; } d }

include!("auto_stubs.rs");

const W: u32 = 1024; const H: u32 = 768;
const CRTC_ID: u32 = 41; const CONN_ID: u32 = 42; const ENC_ID: u32 = 43;

#[repr(C)] #[derive(Clone, Copy)]
pub struct ModeInfo { clock: u32, hdisplay: u16, hsync_start: u16, hsync_end: u16, htotal: u16, hskew: u16, vdisplay: u16, vsync_start: u16, vsync_end: u16, vtotal: u16, vscan: u16, vrefresh: u32, flags: u32, typ: u32, name: [u8; 32] }
const MODE: ModeInfo = ModeInfo { clock: 65000, hdisplay: 1024, hsync_start: 1048, hsync_end: 1184, htotal: 1344, hskew: 0, vdisplay: 768, vsync_start: 771, vsync_end: 777, vtotal: 806, vscan: 0, vrefresh: 60, flags: 0, typ: 72,
    name: *b"1024x768\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0" };

#[repr(C)] pub struct Res { count_fbs: i32, fbs: *mut u32, count_crtcs: i32, crtcs: *mut u32, count_connectors: i32, connectors: *mut u32, count_encoders: i32, encoders: *mut u32, min_width: u32, max_width: u32, min_height: u32, max_height: u32 }
#[repr(C)] pub struct Conn { connector_id: u32, encoder_id: u32, connector_type: u32, connector_type_id: u32, connection: u32, mm_width: u32, mm_height: u32, subpixel: u32, count_modes: i32, modes: *mut ModeInfo, count_props: i32, props: *mut u32, prop_values: *mut u64, count_encoders: i32, encoders: *mut u32 }
#[repr(C)] pub struct Enc { encoder_id: u32, encoder_type: u32, crtc_id: u32, possible_crtcs: u32, possible_clones: u32 }
#[repr(C)] pub struct Crtc { crtc_id: u32, buffer_id: u32, x: u32, y: u32, width: u32, height: u32, mode_valid: i32, mode: ModeInfo, gamma_size: i32 }

static mut CRTCS: [u32; 1] = [CRTC_ID]; static mut CONNS: [u32; 1] = [CONN_ID]; static mut ENCS: [u32; 1] = [ENC_ID];
static mut MODES: [ModeInfo; 1] = [MODE];
static mut RES: Res = Res { count_fbs: 0, fbs: null_mut(), count_crtcs: 1, crtcs: null_mut(), count_connectors: 1, connectors: null_mut(), count_encoders: 1, encoders: null_mut(), min_width: 320, max_width: 4096, min_height: 200, max_height: 4096 };
static mut CONN: Conn = Conn { connector_id: CONN_ID, encoder_id: ENC_ID, connector_type: 11, connector_type_id: 1, connection: 1, mm_width: 150, mm_height: 110, subpixel: 0, count_modes: 1, modes: null_mut(), count_props: 0, props: null_mut(), prop_values: null_mut(), count_encoders: 1, encoders: null_mut() };
static mut ENC: Enc = Enc { encoder_id: ENC_ID, encoder_type: 2, crtc_id: CRTC_ID, possible_crtcs: 1, possible_clones: 0 };
static mut CRTC: Crtc = Crtc { crtc_id: CRTC_ID, buffer_id: 1, x: 0, y: 0, width: W, height: H, mode_valid: 1, mode: MODE, gamma_size: 0 };
static mut FBS: u32 = 0;
static mut PENDING: usize = 0;

#[no_mangle] pub unsafe extern "C" fn drmModeGetResources(_fd: i32) -> *mut Res { RES.crtcs = CRTCS.as_mut_ptr(); RES.connectors = CONNS.as_mut_ptr(); RES.encoders = ENCS.as_mut_ptr(); &mut RES }
#[no_mangle] pub extern "C" fn drmModeFreeResources(_r: *mut Res) {}
#[no_mangle] pub unsafe extern "C" fn drmModeGetConnector(_fd: i32, _id: u32) -> *mut Conn { CONN.modes = MODES.as_mut_ptr(); CONN.encoders = ENCS.as_mut_ptr(); &mut CONN }
#[no_mangle] pub extern "C" fn drmModeFreeConnector(_c: *mut Conn) {}
#[no_mangle] pub unsafe extern "C" fn drmModeGetEncoder(_fd: i32, _id: u32) -> *mut Enc { &mut ENC }
#[no_mangle] pub extern "C" fn drmModeFreeEncoder(_e: *mut Enc) {}
#[no_mangle] pub unsafe extern "C" fn drmModeGetCrtc(_fd: i32, _id: u32) -> *mut Crtc { &mut CRTC }
#[no_mangle] pub extern "C" fn drmModeFreeCrtc(_c: *mut Crtc) {}
#[no_mangle] pub unsafe extern "C" fn drmModeAddFB(_fd: i32, _w: u32, _h: u32, _d: u8, _b: u8, _p: u32, _h2: u32, id: *mut u32) -> i32 { FBS += 1; if !id.is_null() { *id = 100 + FBS; } 0 }
#[no_mangle] pub unsafe extern "C" fn drmModeAddFB2(_fd: i32, _w: u32, _h: u32, _f: u32, _h2: *const u32, _p: *const u32, _o: *const u32, id: *mut u32, _fl: u32) -> i32 { FBS += 1; if !id.is_null() { *id = 100 + FBS; } 0 }
#[no_mangle] pub unsafe extern "C" fn drmModeAddFB2WithModifiers(_fd: i32, _w: u32, _h: u32, _f: u32, _h2: *const u32, _p: *const u32, _o: *const u32, _m: *const u64, id: *mut u32, _fl: u32) -> i32 { FBS += 1; if !id.is_null() { *id = 100 + FBS; } 0 }
#[no_mangle] pub extern "C" fn drmModeRmFB(_fd: i32, _id: u32) -> i32 { 0 }
#[no_mangle] pub extern "C" fn drmModeFreeFB(_p: usize) {}
#[no_mangle] pub extern "C" fn drmModeGetFB(_fd: i32, _id: u32) -> usize { 0 }
#[no_mangle] pub extern "C" fn drmModeSetCrtc(_fd: i32, _c: u32, _fb: u32, _x: u32, _y: u32, _cn: *const u32, _n: i32, _m: *const ModeInfo) -> i32 { 0 }
#[no_mangle] pub unsafe extern "C" fn drmModePageFlip(_fd: i32, _c: u32, _fb: u32, _fl: u32, data: usize) -> i32 { PENDING = data; 0 }
#[no_mangle] pub unsafe extern "C" fn drmHandleEvent(fd: i32, ctx: *const usize) -> i32 {
    if PENDING != 0 {
        let h = read_volatile(ctx.add(2));   // evctx.page_flip_handler (offset 8: version, vblank_handler, page_flip_handler)
        let d = PENDING; PENDING = 0;
        if h != 0 { let f: extern "C" fn(i32, u32, u32, u32, usize) = core::mem::transmute(h); f(fd, 0, 0, 0, d); }
    }
    0
}
#[no_mangle] pub unsafe extern "C" fn drmGetCap(_fd: i32, _cap: u64, val: *mut u64) -> i32 { if !val.is_null() { *val = 1; } 0 }
#[no_mangle] pub extern "C" fn drmSetClientCap(_fd: i32, _cap: u64, _v: u64) -> i32 { 0 }
#[no_mangle] pub extern "C" fn drmSetMaster(_fd: i32) -> i32 { 0 }
#[no_mangle] pub extern "C" fn drmDropMaster(_fd: i32) -> i32 { 0 }
#[no_mangle] pub extern "C" fn drmIoctl(_fd: i32, _r: usize, _a: usize) -> i32 { 0 }
#[no_mangle] pub extern "C" fn drmModeSetCursor(_fd: i32, _c: u32, _h: u32, _w: u32, _hh: u32) -> i32 { 0 }
#[no_mangle] pub extern "C" fn drmModeSetCursor2(_fd: i32, _c: u32, _h: u32, _w: u32, _hh: u32, _x: i32, _y: i32) -> i32 { 0 }
#[no_mangle] pub extern "C" fn drmModeMoveCursor(_fd: i32, _c: u32, _x: i32, _y: i32) -> i32 { 0 }

// ---- gbm ----
#[repr(C)] #[derive(Clone, Copy)] struct Bo { w: u32, h: u32, fmt: u32, user: usize, destroy: usize }
static mut BOS: [Bo; 4] = [Bo { w: W, h: H, fmt: 0x34325258, user: 0, destroy: 0 }; 4];
static mut NEXT_BO: usize = 0;
static mut DEV_FD: i32 = 0;
static mut DEVICE: [u32; 4] = [0; 4];
static mut SURFACE: [u32; 4] = [0; 4];
#[repr(C)] pub struct Handle { lo: u32, hi: u32 }
#[no_mangle] pub unsafe extern "C" fn gbm_create_device(fd: i32) -> *mut u32 { DEV_FD = fd; DEVICE.as_mut_ptr() }
#[no_mangle] pub extern "C" fn gbm_device_destroy(_d: *mut u32) {}
#[no_mangle] pub unsafe extern "C" fn gbm_device_get_fd(_d: *mut u32) -> i32 { DEV_FD }
#[no_mangle] pub extern "C" fn gbm_device_get_backend_name(_d: *mut u32) -> *const u8 { b"glremote\0".as_ptr() }
#[no_mangle] pub extern "C" fn gbm_device_is_format_supported(_d: *mut u32, _f: u32, _fl: u32) -> i32 { 1 }
#[no_mangle] pub unsafe extern "C" fn gbm_surface_create(_d: *mut u32, w: u32, h: u32, f: u32, _fl: u32) -> *mut u32 { let mut i = 0; while i < 4 { BOS[i].w = w; BOS[i].h = h; BOS[i].fmt = f; i += 1; } SURFACE.as_mut_ptr() }
#[no_mangle] pub extern "C" fn gbm_surface_destroy(_s: *mut u32) {}
#[no_mangle] pub unsafe extern "C" fn gbm_surface_lock_front_buffer(_s: *mut u32) -> *mut Bo { NEXT_BO = (NEXT_BO + 1) & 1; &mut BOS[NEXT_BO] }
#[no_mangle] pub extern "C" fn gbm_surface_release_buffer(_s: *mut u32, _b: *mut Bo) {}
#[no_mangle] pub extern "C" fn gbm_surface_has_free_buffers(_s: *mut u32) -> i32 { 1 }
#[no_mangle] pub unsafe extern "C" fn gbm_bo_create(_d: *mut u32, w: u32, h: u32, f: u32, _fl: u32) -> *mut Bo { BOS[2] = Bo { w, h, fmt: f, user: 0, destroy: 0 }; &mut BOS[2] }
#[no_mangle] pub unsafe extern "C" fn gbm_bo_destroy(b: *mut Bo) { let d = (*b).destroy; if d != 0 && (*b).user != 0 { let f: extern "C" fn(*mut Bo, usize) = core::mem::transmute(d); f(b, (*b).user); } (*b).user = 0; (*b).destroy = 0; }
#[no_mangle] pub extern "C" fn gbm_bo_write(_b: *mut Bo, _buf: *const u8, _n: usize) -> i32 { 0 }
#[no_mangle] pub unsafe extern "C" fn gbm_bo_get_handle(_b: *mut Bo) -> Handle { Handle { lo: 1, hi: 0 } }
#[no_mangle] pub unsafe extern "C" fn gbm_bo_get_handle_for_plane(_b: *mut Bo, _p: i32) -> Handle { Handle { lo: 1, hi: 0 } }
#[no_mangle] pub unsafe extern "C" fn gbm_bo_get_stride(b: *mut Bo) -> u32 { (*b).w * 4 }
#[no_mangle] pub unsafe extern "C" fn gbm_bo_get_stride_for_plane(b: *mut Bo, _p: i32) -> u32 { (*b).w * 4 }
#[no_mangle] pub unsafe extern "C" fn gbm_bo_get_width(b: *mut Bo) -> u32 { (*b).w }
#[no_mangle] pub unsafe extern "C" fn gbm_bo_get_height(b: *mut Bo) -> u32 { (*b).h }
#[no_mangle] pub unsafe extern "C" fn gbm_bo_get_format(b: *mut Bo) -> u32 { (*b).fmt }
#[no_mangle] pub unsafe extern "C" fn gbm_bo_get_user_data(b: *mut Bo) -> usize { (*b).user }
#[no_mangle] pub unsafe extern "C" fn gbm_bo_set_user_data(b: *mut Bo, d: usize, destroy: usize) { (*b).user = d; (*b).destroy = destroy; }
#[no_mangle] pub unsafe extern "C" fn gbm_bo_get_device(_b: *mut Bo) -> *mut u32 { DEVICE.as_mut_ptr() }
#[no_mangle] pub extern "C" fn gbm_bo_get_offset(_b: *mut Bo, _p: i32) -> u32 { 0 }
#[no_mangle] pub extern "C" fn gbm_bo_get_plane_count(_b: *mut Bo) -> i32 { 1 }
#[no_mangle] pub extern "C" fn gbm_bo_get_modifier(_b: *mut Bo) -> u64 { 0 }
#[no_mangle] pub extern "C" fn gbm_bo_get_fd(_b: *mut Bo) -> i32 { -1 }
