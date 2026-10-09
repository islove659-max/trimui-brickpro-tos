# Kế hoạch (nháp, 2026-10-08)

## Giai đoạn 0 — Khảo sát máy thật (làm trước, không rủi ro)
Qua SSH, lưu vào `tu_lieu_may_that/`:
- `uname -a`, `cat /proc/cpuinfo /proc/meminfo /proc/mounts /proc/partitions /proc/cmdline`
- `ps`, `lsmod`, `ls -l /lib /usr/lib /usr/trimui/bin`, `ldd --version`
- `/etc/init.d`, `/etc/inittab`, `runtrimui`, danh sách service
- `amixer controls`, `/proc/bus/input/devices`, `/sys/class/*` quan trọng
- Thư viện có sẵn: SDL2? GLES/EGL? libdrm? alsa?
Đã có sẵn một phần ở `C:\Users\X\dự án chỉnh sửa lại flimware\tu_lieu_may_that_20261007\`.

## Giai đoạn 1 — Lớp tương thích (overlay, đo được từng bước)
1. `/run` (tmpfs) + thư mục runtime chuẩn; kiểm tra không phá `wlan0`.
2. Thiết bị input cho libinput/Weston (mdev hoặc script gắn tag) thay cho hack `require-input=false`.
3. Thư viện chung (SDL2, GLES, ALSA, libdrm) để app ngoài không phải tự mang.
4. Chuẩn CPU profile (balanced/performance/battery) dùng chung, tự khôi phục khi thoát.

## Giai đoạn 2 — SDK và đóng gói app
- Toolchain Docker `messense/rust-musl-cross:aarch64-musl` + sysroot glibc 2.33 cho app C.
- Một manifest app duy nhất (`app.json`) → script tự sinh cho Stock/Knulli/Spruce/NextUI.
- App mẫu: framebuffer + evdev + âm thanh (lấy từ các module đã sửa trong CLAUDE.md của dự án thẻ).

## Giai đoạn 3 — Launcher/MainUI mở rộng
- Quản lý runtime, quyền, cập nhật OTA an toàn (sha256, ELF check, rollback).
- Quyết định: viết launcher mới hay mở rộng MainUI (chưa có mã nguồn MainUI → có thể thay bằng launcher riêng gọi qua `runtrimui`).

## Giai đoạn 4 — Build firmware và nạp
- Dựa trên `build_firmware.py` / `ext4lite.py` của dự án firmware cũ.
- Quy trình: test busybox trên PC → build img → người dùng nạp → thu log về.
- Điều kiện bắt buộc: không swap, giữ sync thẻ SD, có thẻ cứu hộ.

## Câu hỏi mở
- "Đa nền tảng" nghĩa là: (a) OS này chạy trên nhiều máy TrimUI (Smart Pro, Brick, Brick Hammer…), (b) app chạy được trên nhiều CFW, hay (c) cả hai?
- Có muốn thay hẳn rootfs (Buildroot/Yocto, kernel 4.9 giữ nguyên) hay chỉ lớp phủ lên Stock?
