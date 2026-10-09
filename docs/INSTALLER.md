# Bộ cài TOS 4.0 (cài trên máy, không cần nạp firmware)

Gói `TOS-Installer-4.0.zip` giải nén vào gốc thẻ SD, rồi mở app **"TOS - Cai dat / Cap nhat"** trên máy. Mọi thứ cài một lần.

## Cài những gì
| Thành phần | Chi tiết |
|---|---|
| Runtime ARMHF 32-bit | glibc 2.31, libgcc, libstdc++ của Debian 11 → `/lib/arm-linux-gnueabihf`, trình nạp `/lib/ld-linux-armhf.so.3` (PortMaster tự nhận máy có ARMHF) |
| glremote | `/usr/bin/glremote_run`, `/usr/lib/glremote/…`: chạy game 32-bit bằng GPU PowerVR thật + âm thanh |
| Luật udev | nhận tay cầm/thiết bị đầu vào cho các port dùng Weston/libinput |
| sysctl | `vfs_cache_pressure=50`, `inotify=65536`, `printk`, `rmem_max` |
| Chế độ game | `tos_gamemode.sh` + 2 dòng chèn vào `preload.sh` / `premainui.sh`: vào game thì tạm dừng `sftpgo`/`MtpDaemon`, thoát game thì bật lại |
| `/run` | symlink `/run → /var/run` (libseat/Weston) |

Chưa áp dụng: `loglevel=4` của nhân (chỉ có trong bản firmware nạp đầy đủ).

## An toàn
* Chỉ ghi vào tầng ghi (overlay) của hệ thống; **không** đụng bootloader, nhân, boot-resource, recovery, cũng không xoá dữ liệu hay thẻ game.
* Trước khi chép: kiểm tra `aarch64`, đúng Stock OS TrimUI, còn ≥15 MB, và SHA256 của từng tệp trong gói. Sai là dừng, không chép gì.
* Tệp hệ thống bị ghi đè được lưu tại `/mnt/UDISK/tos/backup`; hai file `preload.sh`/`premainui.sh` chỉ thêm một khối có đánh dấu `# [tos-begin]…# [tos-end]` (có sẵn thì bỏ qua).
* **Gỡ**: app **"TOS - Go cai dat"** xoá đúng các tệp đã cài và trả hai hook về nguyên bản. Chạy lại "Cai dat" để cập nhật (không làm gì thêm nếu không có gì mới).
* Nhật ký: `System/tos/install.log` trên thẻ SD.

## Kiểm thử (máy ảo QEMU, rootfs thật của hãng)
`tools/vm_guest_installer.sh` — trên rootfs Stock gốc: 27/27 đạt (cài, chạy game 32-bit qua glremote, chế độ game bằng hook thật, cài lần 2 không đổi gì, gỡ trả hook về bản gốc từng byte và xoá sạch, cài lại). Trên rootfs đã có TOS (tos-4.0): 24/24 đạt (cài/gỡ không phá tệp của firmware).

## Dựng gói
`powershell -File armhf_gl/build_runtime.ps1` rồi `python installer/build_installer.py` → `out/pkg/TOS-Installer-4.0.zip` và bản kèm PortMaster gốc.
