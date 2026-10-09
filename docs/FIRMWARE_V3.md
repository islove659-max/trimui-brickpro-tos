# Firmware v3 (tos-3.0) = v2 + runtime ARMHF 32-bit — 2026-10-08

Ảnh: `out/fw_v3/sd_recovery_tg4040_brickpro_ver1.1.1_20260717_tos3.img` (SHA256 trong file `.sha256` cạnh nó).
Build: `python -I -X utf8 tools/build_fw_v2.py --armhf C:\Users\X\tools\armhf_x` (nguồn ARMHF do `tools/lay_armhf.py` trích từ 3 gói Debian). Kiểm toán: `tools/kiem_toan_rootfs.py`.

## Nguồn ARMHF (đã tải với sự đồng ý của người dùng, từ `deb.debian.org`, Debian 11 "bullseye")
| Gói | Kích thước | Đối chiếu |
|---|---|---|
| `libc6_2.31-13+deb11u11_armhf.deb` | 2.331.296 B | SHA256 + kích thước KHỚP chỉ mục `Packages` chính thức của bullseye |
| `libgcc-s1_10.2.1-6_armhf.deb` | 36.200 B | KHỚP |
| `libstdc++6_10.2.1-6_armhf.deb` | 420.924 B | KHỚP |
Chỉ trích thư viện thời chạy (15 file + 16 symlink, 3,09MB): `ld-2.31.so`, `libc`, `libm`, `libpthread`, `libdl`, `librt`, `libutil`, `libresolv`, `libanl`, `libnsl`, `libnss_files/dns`, `libBrokenLocale`, `libgcc_s.so.1`, `libstdc++.so.6.0.28`. Không lấy locale/gconv/tài liệu/công cụ.

## Bố cục trong rootfs (chuẩn Debian multiarch, không đụng thư viện 64-bit)
- `/lib/ld-linux-armhf.so.3 -> arm-linux-gnueabihf/ld-2.31.so` (đây là đường dẫn trình nạp mà ELF ARMHF yêu cầu)
- `/lib/arm-linux-gnueabihf/*` và `/usr/lib/arm-linux-gnueabihf/libstdc++.so.6*`
- rootfs dùng thêm 3.162KB, còn trống 90MB; fsck sạch.

## Kiểm chứng
- Kiểm toán: gốc 4077 mục -> mới 4117, MẤT: không, MỚI: 40, ĐỔI: 2 (hai script móc).
- Bootloader/kernel/boot-resource/recovery giống hệt thẻ gốc từng byte; mọi checksum V*.fex + CRC env đúng.
- **Máy ảo QEMU: 38/38 đạt.** Ngoài 32 kiểm tra của v2, có 6 kiểm tra ARMHF: symlink loader đúng, `ld-2.31.so` là ELF 32-bit ARM (e_machine=40), đủ thư viện lõi, **chạy được `libc.so.6` 32-bit qua `/lib/ld-linux-armhf.so.3` (in "GNU C Library (Debian GLIBC 2.31-13+deb11u11) stable release version 2.31")**, loader tự chạy (`--list`), và 64-bit không bị ảnh hưởng.

## Chưa kiểm chứng
Trên máy thật chưa chạy chương trình ARMHF nào (cần flash). Sau flash: chạy thử `gptokeyb.armhf` của PortMaster; nó phải chạy tới lỗi thiếu thư viện khác (nếu có) thay vì `not found`. Thư viện bổ sung của từng port (SDL2, GLES 32-bit...) do port hoặc runtime của nó mang theo (ví dụ `lib_armhf` trong `weston_pkg`).

---
# Kết quả sau khi nạp v3 lên máy thật (2026-10-09 ~00:23 giờ máy) — `out/fw_v3/kiem_tra_sau_flash.txt`
- `/etc/tos_version`: `tos-3.0`, `armhf: 1`. 19/19 kiểm tra đạt (thay đổi ở tầng /rom; sysctl, cmdline loglevel=4, libinput thấy thiết bị, không swap, thẻ SD sync).
- **ARMHF trên máy thật:** `/lib/ld-linux-armhf.so.3` có; chạy được `libc.so.6` 32-bit (in "GNU C Library (Debian GLIBC 2.31-13+deb11u11) stable release version 2.31"); **`7zzs.armhf` của PortMaster chạy được ("7-Zip 24.09 (armt)")**; `gptokeyb.armhf` hết báo `not found` và giờ báo `libSDL2-2.0.so.0: wrong ELF class: ELFCLASS64` (đúng kỳ vọng: nó cần libSDL2 32-bit mà runtime/port ARMHF phải tự mang theo; hệ thống chỉ có libSDL2 64-bit).
- Quét nhật ký: `logread` không có lỗi; các dòng error/fail trong `dmesg` (VE debug register, mmc gpios, opp debugfs -12, usb_detect_mode, pvr sunxi_decide_pll, configfs-gadget -19) đều là của driver Stock, đã có sẵn trước khi flash.
