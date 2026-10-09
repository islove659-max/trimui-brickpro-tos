//! Framebuffer /dev/fb0: đọc thông số thật bằng ioctl, vẽ vào trang ẩn rồi FBIOPAN_DISPLAY.
use crate::sys::*;
use std::fmt;

const FBIOGET_VSCREENINFO: i32 = 0x4600;
const FBIOPUT_VSCREENINFO: i32 = 0x4601;
const FBIOGET_FSCREENINFO: i32 = 0x4602;
const FBIOPAN_DISPLAY: i32 = 0x4606;
const FBIO_WAITFORVSYNC: i32 = ioc(1, b'F', 0x20, 4);

#[repr(C)]
#[derive(Default, Clone, Copy)]
struct Bitfield { offset: u32, length: u32, msb_right: u32 }

#[repr(C)]
#[derive(Default, Clone, Copy)]
struct VarInfo {
    xres: u32, yres: u32, xres_virtual: u32, yres_virtual: u32, xoffset: u32, yoffset: u32,
    bits_per_pixel: u32, grayscale: u32,
    red: Bitfield, green: Bitfield, blue: Bitfield, transp: Bitfield,
    nonstd: u32, activate: u32, height: u32, width: u32, accel_flags: u32,
    pixclock: u32, left_margin: u32, right_margin: u32, upper_margin: u32, lower_margin: u32,
    hsync_len: u32, vsync_len: u32, sync: u32, vmode: u32, rotate: u32, colorspace: u32,
    reserved: [u32; 4],
}

#[repr(C)]
#[derive(Default, Clone, Copy)]
struct FixInfo {
    id: [u8; 16], smem_start: u64, smem_len: u32, type_: u32, type_aux: u32, visual: u32,
    xpanstep: u16, ypanstep: u16, ywrapstep: u16, line_length: u32,
    mmio_start: u64, mmio_len: u32, accel: u32, capabilities: u16, reserved: [u16; 2],
}

#[derive(Debug)]
pub enum FbError { Open(String), Ioctl(&'static str), Format(u32), Mmap }
impl fmt::Display for FbError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            FbError::Open(p) => write!(f, "không mở được {}", p),
            FbError::Ioctl(n) => write!(f, "ioctl {} lỗi", n),
            FbError::Format(b) => write!(f, "bits_per_pixel={} chưa hỗ trợ (cần 32)", b),
            FbError::Mmap => write!(f, "mmap lỗi"),
        }
    }
}

pub struct Fb {
    fd: i32, map: *mut u8, map_len: usize,
    pub width: u32, pub height: u32, pub stride: usize, // stride tính bằng byte
    pages: u32, cur: u32, off: [u32; 3], // offset r,g,b
    var: VarInfo,
    pub vsync_ok: bool,
}

impl Fb {
    pub fn open(path: &str) -> Result<Fb, FbError> {
        unsafe {
            let p = c_path(path);
            let fd = open(p.as_ptr(), O_RDWR);
            if fd < 0 { return Err(FbError::Open(path.to_string())); }
            let mut v = VarInfo::default();
            let mut x = FixInfo::default();
            if ioctl(fd, FBIOGET_VSCREENINFO, &mut v as *mut VarInfo) < 0 { return Err(FbError::Ioctl("GET_VSCREENINFO")); }
            if ioctl(fd, FBIOGET_FSCREENINFO, &mut x as *mut FixInfo) < 0 { return Err(FbError::Ioctl("GET_FSCREENINFO")); }
            if v.bits_per_pixel != 32 { return Err(FbError::Format(v.bits_per_pixel)); }
            let map_len = x.smem_len as usize;
            let map = mmap(std::ptr::null_mut(), map_len, PROT_READ | PROT_WRITE, MAP_SHARED, fd, 0);
            if map as isize == -1 { return Err(FbError::Mmap); }
            let stride = x.line_length as usize;
            // Số trang thật = min(yres_virtual, smem_len / (stride*yres)); chỉ cần 2 trang.
            let fit = (map_len / (stride * v.yres as usize)) as u32;
            let pages = (v.yres_virtual / v.yres).min(fit).max(1).min(2);
            Ok(Fb {
                fd, map: map as *mut u8, map_len, width: v.xres, height: v.yres, stride,
                pages, cur: 0, off: [v.red.offset, v.green.offset, v.blue.offset], var: v, vsync_ok: true,
            })
        }
    }
    pub fn pages(&self) -> u32 { self.pages }
    /// Offset bit của kênh R,G,B trong mỗi pixel (đọc từ fb_var_screeninfo).
    pub fn channel_offsets(&self) -> [u32; 3] { self.off }
    /// Màu ARGB theo offset kênh của fb (đúng cho mọi thứ tự RGB/BGR).
    pub fn rgb(&self, r: u8, g: u8, b: u8) -> u32 {
        ((r as u32) << self.off[0]) | ((g as u32) << self.off[1]) | ((b as u32) << self.off[2])
    }
    /// Trang đang ẩn (vẽ vào đây).
    pub fn back(&mut self) -> &mut [u32] {
        let page = if self.pages > 1 { 1 - self.cur } else { 0 };
        let words = self.stride / 4 * self.height as usize;
        unsafe { std::slice::from_raw_parts_mut(self.map.add(page as usize * self.stride * self.height as usize) as *mut u32, words) }
    }
    pub fn stride_px(&self) -> usize { self.stride / 4 }
    /// Lật trang: pan sang trang vừa vẽ + chờ vsync nếu driver hỗ trợ. Nếu chỉ có 1 trang thì không làm gì.
    pub fn flip(&mut self) {
        if self.pages < 2 { return; }
        let next = 1 - self.cur;
        let mut v = self.var;
        v.yoffset = next * self.height;
        unsafe {
            if ioctl(self.fd, FBIOPAN_DISPLAY, &mut v as *mut VarInfo) == 0 {
                self.cur = next;
                if self.vsync_ok {
                    let mut arg: u32 = 0;
                    if ioctl(self.fd, FBIO_WAITFORVSYNC, &mut arg as *mut u32) < 0 { self.vsync_ok = false; }
                }
            }
        }
    }
    pub fn clear_and_reset(&mut self) {
        unsafe { std::ptr::write_bytes(self.map, 0, self.map_len); }
        let mut v = self.var; v.yoffset = 0;
        unsafe { ioctl(self.fd, FBIOPAN_DISPLAY, &mut v as *mut VarInfo); }
        self.cur = 0;
    }
}
impl Drop for Fb {
    fn drop(&mut self) {
        self.clear_and_reset();
        unsafe { munmap(self.map as *mut _, self.map_len); close(self.fd); }
    }
}
#[allow(dead_code)] const _UNUSED: i32 = FBIOPUT_VSCREENINFO;
