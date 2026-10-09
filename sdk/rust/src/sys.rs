//! Khai báo thủ công các hàm libc (musl tĩnh đã được std liên kết sẵn) để khỏi cần crate `libc`.
use std::os::raw::{c_int, c_long, c_ulong, c_void};

pub const O_RDWR: c_int = 2;
pub const O_RDONLY: c_int = 0;
pub const O_NONBLOCK: c_int = 0o4000; // aarch64
pub const PROT_READ: c_int = 1;
pub const PROT_WRITE: c_int = 2;
pub const MAP_SHARED: c_int = 1;
pub const SIGINT: c_int = 2;
pub const SIGTERM: c_int = 15;

extern "C" {
    pub fn open(path: *const u8, flags: c_int, ...) -> c_int;
    pub fn close(fd: c_int) -> c_int;
    pub fn read(fd: c_int, buf: *mut c_void, n: usize) -> isize;
    pub fn ioctl(fd: c_int, req: c_int, ...) -> c_int;
    pub fn mmap(addr: *mut c_void, len: usize, prot: c_int, flags: c_int, fd: c_int, off: c_long) -> *mut c_void;
    pub fn munmap(addr: *mut c_void, len: usize) -> c_int;
    pub fn signal(sig: c_int, handler: usize) -> usize;
}

/// Dựng mã ioctl kiểu _IOC (aarch64). dir: 1=write, 2=read.
pub const fn ioc(dir: u32, ty: u8, nr: u32, size: u32) -> c_int {
    ((dir << 30) | (size << 16) | ((ty as u32) << 8) | nr) as c_int
}

pub fn c_path(s: &str) -> Vec<u8> {
    let mut v = s.as_bytes().to_vec();
    v.push(0);
    v
}

#[allow(dead_code)]
pub type CUlong = c_ulong;
