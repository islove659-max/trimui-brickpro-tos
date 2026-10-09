#![no_std]
use core::panic::PanicInfo;
#[panic_handler]
fn panic(_: &PanicInfo) -> ! { loop {} }

// ARM EHABI: bảng unwind tham chiếu các hàm personality; không dùng libc nên tự cung cấp bản rỗng (panic=abort, không bao giờ gọi).
#[no_mangle] pub extern "C" fn __aeabi_unwind_cpp_pr0() {}
#[no_mangle] pub extern "C" fn __aeabi_unwind_cpp_pr1() {}
#[no_mangle] pub extern "C" fn __aeabi_unwind_cpp_pr2() {}

/// Hàm thử: có trong thư viện chia sẻ 32-bit không dùng libc.
#[no_mangle]
pub extern "C" fn ctest_add(a: i32, b: i32) -> i32 { a + b }