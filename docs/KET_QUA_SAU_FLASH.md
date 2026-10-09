# Kết quả sau khi nạp firmware TOS v2 lên máy thật (2026-10-08 ~00:04 giờ máy)

Thẻ nạp: 128GB (Disk 2), ghi 2.560MB, đọc lại SHA256 khớp ảnh `07bea2c9…c4dab7`. Máy khởi động bình thường, kết nối Wi-Fi, SSH bằng khoá `trimui_brick_the2` (host key mới, file `known_hosts_trimui_3`).

## Kiểm tra tĩnh trên máy: 21/21 đạt (`out/fw_v2/kiem_tra_sau_flash.txt`)
- `/etc/tos_version`: `tos-2.0`, build 22:45 (đúng ảnh đã nạp).
- Thay đổi nằm ở TẦNG ROOTFS CỦA FIRMWARE (`/rom`): luật udev, sysctl.d, `tos_gamemode.sh`, `tos_env.sh`, symlink `/run`, móc `[tos]` trong `preload.sh`/`premainui.sh`. Overlay không còn che luật udev.
- `sysctl`: printk `4 4 1 7`, inotify watches 65536, vfs_cache_pressure 50, rmem_max 1048576.
- `cmdline`: `loglevel=4 loglevel=4` (env u-boot đã áp dụng, cả hai lần).
- `libinput-list-devices` thấy 3 thiết bị (Audio Jack, sunxi-keyboard, TRIMUI Player1); `event3` có `ID_INPUT_JOYSTICK=1`.
- Không có thay đổi rủi ro: không swap, thẻ SD và UDISK vẫn mount `sync`, `kernel.panic=3` (mặc định hãng).
- Busybox đang dùng: v1.36.1 (thẻ SD/overlay đặt lên lại sau flash). `tos_gamemode.sh` không dùng `find -delete`.

## Kiểm tra chức năng chạy thật
Dựng lại đường mở app (MainUI thoát → `runtrimui` chạy `/tmp/cmd_to_run.sh`): trong lúc app chạy `sftpgo`, `MtpDaemon`, `MainUI` đều dừng; `osdd` và `keymon` còn sống; `PATH` đúng; chế độ game báo ON. Sau khi app xong: hai dịch vụ chạy lại, MainUI quay lại, chế độ game về OFF. `tos_gm.log` ghi 6 chu kỳ ON/OFF liên tiếp, kể cả các lần người dùng tự mở app.

## Còn lại
- Cài đặt MainUI bị xoá sau flash (đúng dự kiến): `osdshortcut=0` (nút Home hông hoạt động), `vol=14`, `mute=1`, ngôn ngữ vn.
- ARMHF 32-bit chưa đưa vào firmware (chờ đồng ý tải 3 gói Debian).
