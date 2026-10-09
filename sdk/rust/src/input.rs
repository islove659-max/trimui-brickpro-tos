//! evdev: tự quét /dev/input/event*, chọn thiết bị có BTN_SOUTH/BTN_EAST (304/305), đọc không chặn.
use crate::sys::*;

pub const BTN_B: u16 = 304; // nhãn B trên máy (Linux: BTN_SOUTH)
pub const BTN_A: u16 = 305; // nhãn A (BTN_EAST)
pub const BTN_Y: u16 = 307; // nhãn Y (BTN_NORTH)
pub const BTN_X: u16 = 308; // nhãn X (BTN_WEST)
pub const BTN_MENU: u16 = 316;
pub const KEY_UP: u16 = 103;
pub const KEY_LEFT: u16 = 105;
pub const KEY_RIGHT: u16 = 106;
pub const KEY_DOWN: u16 = 108;

const EV_KEY: u16 = 1;
const EV_ABS: u16 = 3;
const ABS_HAT0X: u16 = 16;
const ABS_HAT0Y: u16 = 17;

#[repr(C)]
#[derive(Clone, Copy)]
struct InputEvent { sec: i64, usec: i64, type_: u16, code: u16, value: i32 } // 24 byte trên aarch64

#[derive(Debug, Clone, Copy)]
pub struct Key { pub code: u16, pub down: bool, pub repeat: bool }

pub struct Pads { fds: Vec<i32> }

fn test_bit(buf: &[u8], bit: usize) -> bool { buf.get(bit / 8).map_or(false, |b| b & (1 << (bit % 8)) != 0) }

impl Pads {
    pub fn scan() -> Pads {
        let mut fds = Vec::new();
        for i in 0..16 {
            let path = c_path(&format!("/dev/input/event{}", i));
            unsafe {
                let fd = open(path.as_ptr(), O_RDONLY | O_NONBLOCK);
                if fd < 0 { continue; }
                // EVIOCGBIT(EV_KEY, 96 byte): đủ tới bit 767
                let mut bits = [0u8; 96];
                let req = ioc(2, b'E', 0x20 + EV_KEY as u32, bits.len() as u32);
                let ok = ioctl(fd, req, bits.as_mut_ptr()) >= 0;
                // Giữ thiết bị có nút gamepad (304/305) hoặc nút MENU (316) để không bỏ sót nút phụ.
                if ok && (test_bit(&bits, 304) || test_bit(&bits, 305) || test_bit(&bits, 316)) { fds.push(fd); } else { close(fd); }
            }
        }
        Pads { fds }
    }
    pub fn count(&self) -> usize { self.fds.len() }
    /// Đọc hết hàng đợi (tới EAGAIN), D-Pad dạng ABS_HAT quy về 103/108/105/106.
    pub fn poll(&mut self, out: &mut Vec<Key>) {
        let sz = std::mem::size_of::<InputEvent>();
        let mut buf = [InputEvent { sec: 0, usec: 0, type_: 0, code: 0, value: 0 }; 64];
        for &fd in &self.fds {
            loop {
                let n = unsafe { read(fd, buf.as_mut_ptr() as *mut _, sz * buf.len()) };
                if n <= 0 { break; }
                for e in &buf[..(n as usize / sz)] {
                    match e.type_ {
                        EV_KEY => out.push(Key { code: e.code, down: e.value != 0, repeat: e.value == 2 }),
                        EV_ABS if e.code == ABS_HAT0X => {
                            out.push(Key { code: KEY_LEFT, down: e.value < 0, repeat: false });
                            out.push(Key { code: KEY_RIGHT, down: e.value > 0, repeat: false });
                        }
                        EV_ABS if e.code == ABS_HAT0Y => {
                            out.push(Key { code: KEY_UP, down: e.value < 0, repeat: false });
                            out.push(Key { code: KEY_DOWN, down: e.value > 0, repeat: false });
                        }
                        _ => {}
                    }
                }
            }
        }
    }
}
impl Drop for Pads { fn drop(&mut self) { for &fd in &self.fds { unsafe { close(fd); } } } }
