# Firmware v2 (tos-2.0) — trạng thái 2026-10-08

Ảnh: `out/fw_v2/sd_recovery_tg4040_brickpro_ver1.1.1_20260717_tos2.img` (SHA256 trong file `.sha256` cạnh nó).
Build: `tools/build_fw_v2.py` (sửa ext4 tại chỗ bằng ext4lite của dự án firmware cũ). Kiểm toán: `tools/kiem_toan_rootfs.py`.

## Thay đổi so với thẻ cứu hộ gốc 1.1.1-20260717
- rootfs **+7 mục**: luật udev input `61-trimui-input.rules`, `sysctl.d/90-trimui-tuning.conf`, symlink `/run -> /var/run`, `tos_gamemode.sh`, `/usr/lib/tos/tos_env.sh`, `/usr/lib/tos/`, `/etc/tos_version`.
- rootfs **đổi 2 script**: `preload.sh`, `premainui.sh` (thêm móc `[tos]` gọi chế độ game).
- env u-boot: `loglevel=8 -> 4` (tuỳ chọn khi build: `--khong-loglevel` để giữ).
- Không mất file nào; bootloader/kernel/boot-resource/recovery **giống hệt từng byte**; mọi checksum `V*.fex` và CRC env đúng.
- **KHÔNG có:** swap, bỏ sync thẻ SD, đổi fstab, kernel.panic (bài học bản v1 gây reset + hỏng FAT).
- **ARMHF 32-bit chưa đưa vào** (chờ người dùng đồng ý tải 3 gói Debian).

## Kiểm chứng đã làm
1. Kiểm toán cây rootfs: 4077 -> 4084 mục, MẤT: không, MỚI: 7, ĐỔI: 2.
2. busybox-w32 `sh -n` cho 6 script.
3. **Máy ảo QEMU (`tools/vm_test.py` + `tools/vm_guest_tests.sh`): 32/32 đạt.** Nhân Linux 6.12 (Alpine virt) gắn rootfs v2 ext4 chỉ đọc, chroot chạy nhị phân thật của Brick Pro: `udevd`/`udevadm` đọc luật TOS không lỗi cú pháp, `sysctl` áp dụng đúng 4 giá trị, 5 script qua `sh -n`, chế độ game với nhị phân thật (vào game dừng 2 dịch vụ, về menu bật lại, không có `cmd_to_run.sh` thì không dừng, `GAMEMODE=0` tắt hẳn), `tos_env.sh` nạp được.
4. Các thay đổi này (dạng overlay) đã chạy trên máy thật qua nhiều lần khởi động lại; luật udev đã kiểm bằng `libinput` thật.

## Phát hiện nhờ máy ảo
Busybox trong rootfs gốc là **v1.27.2** và `find` của nó **không có `-delete`** (máy thật chạy busybox mới do thẻ SD/overlay đặt vào). `tos_gamemode.sh` đã sửa dùng vòng lặp `rm -f` thay `find -delete`.

## Máy ảo KHÔNG kiểm được
Bootloader/nhân/phần cứng A133, GPU, màn hình, âm thanh, tay cầm; khớp luật udev với thiết bị input thật (nhân Alpine không gán driver `virtio_input`).

## Nạp
Xem `ghi_the_sd.ps1` của dự án firmware cũ (chốt an toàn serial + dung lượng + USB). Cần: thẻ SD trống làm thẻ nạp, PowerShell quyền Administrator, người dùng xác nhận đúng ổ đĩa. Nạp xoá toàn bộ bộ nhớ trong của máy (eraseflag=1); thẻ game không bị ảnh hưởng. Quay về bản gốc bằng thẻ cứu hộ gốc.
