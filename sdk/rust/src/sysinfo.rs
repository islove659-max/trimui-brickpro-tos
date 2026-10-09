//! Trạng thái hệ thống (âm lượng, tắt tiếng, độ sáng) đọc từ vùng shared memory của MainUI/keymon.
//! SysV shm key 1313819979, 112 byte, int32 LE. Offset đã XÁC NHẬN bằng thực nghiệm trên Brick Pro Stock 1.1.1:
//!   0x00 vol (0..20) | 0x04 brightness | 0x0c mute (1 = tắt tiếng; đảo bằng `shmvar mute`)
//! Chỉ ĐỌC (SHM_RDONLY); không ghi để khỏi tranh chấp với hardwareservice/keymon.
use std::os::raw::{c_int, c_void};

extern "C" {
    fn shmget(key: c_int, size: usize, flag: c_int) -> c_int;
    fn shmat(id: c_int, addr: *const c_void, flag: c_int) -> *mut c_void;
    fn shmdt(addr: *const c_void) -> c_int;
}
const KEY: c_int = 1_313_819_979;
const SHM_RDONLY: c_int = 0o10000;

#[derive(Debug, Clone, Copy)]
pub struct SysState {
    pub volume: i32,
    pub brightness: i32,
    pub muted: bool,
}

/// Đọc trạng thái hiện tại; None nếu vùng shm chưa tồn tại (MainUI chưa chạy).
pub fn read() -> Option<SysState> {
    unsafe {
        let id = shmget(KEY, 112, 0o444);
        if id < 0 { return None; }
        let p = shmat(id, std::ptr::null(), SHM_RDONLY);
        if p as isize == -1 { return None; }
        let a = p as *const i32;
        let s = SysState { volume: *a, brightness: *a.add(1), muted: *a.add(3) != 0 };
        shmdt(p);
        Some(s)
    }
}
