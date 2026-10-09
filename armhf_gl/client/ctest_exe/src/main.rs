#![no_std]
#![no_main]
use core::arch::asm;
use core::panic::PanicInfo;
#[panic_handler]
fn panic(_: &PanicInfo) -> ! { loop {} }

#[link(name = "ctest_lib")]
extern "C" { fn ctest_add(a: i32, b: i32) -> i32; }

// Lời gọi hệ thống ARM EABI: r7 = số, r0..r2 = tham số, `svc 0`.
unsafe fn sys3(n: u32, a: u32, b: u32, c: u32) -> i32 {
    let ret: i32;
    asm!("svc 0", in("r7") n, inlateout("r0") a => ret, in("r1") b, in("r2") c, options(nostack));
    ret
}

#[no_mangle]
pub unsafe extern "C" fn _start() -> ! {
    let v = ctest_add(40, 2);
    let msg = if v == 42 { b"ARMHF no-libc OK: ctest_add(40,2)=42 (thu vien chia se 32-bit nap qua ld-linux-armhf)\n" as &[u8] } else { b"ARMHF LOI\n" as &[u8] };
    sys3(4, 1, msg.as_ptr() as u32, msg.len() as u32);   // write(1, msg, len)
    sys3(1, if v == 42 { 0 } else { 1 }, 0, 0);          // exit
    loop {}
}