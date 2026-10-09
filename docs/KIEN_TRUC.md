# Kiến trúc nền (chốt 2026-10-08): "đa nền tảng" = nhiều máy TrimUI **và** nhiều CFW

## Nguyên tắc
1. **Một binary, một launcher, bốn gói.** Binary Rust `aarch64-unknown-linux-musl` tĩnh; launcher là shell dùng chung `tos_env.sh`; `package_app.ps1` sinh 4 zip (Stock, Knulli, Spruce, NextUI) từ một thư mục + `app.json`.
2. **Không đoán theo tên máy.** Độ phân giải đọc từ fb (`/sys/class/graphics/fb0/modes`, ioctl trong binary). Máy nhận diện: 1024x768 → `brick`, 1280x720 → `smartpro`, khác → `unknown` (app vẫn chạy theo kích thước thật).
3. **Nền hệ thống (rootfs overlay) chỉ làm việc chung cho mọi app** (udev input, `/run`, sysctl, thư viện chung); phần riêng từng CFW nằm ở `tos_env.sh`.
4. **Mọi thay đổi hệ thống phải khôi phục được:** CPU governor lưu/trả đúng giá trị cũ, trap đặt trước khi STOP MainUI, `cleanup` chạy một lần.

## Thành phần
| Thành phần | Vị trí | Trạng thái |
|---|---|---|
| Lớp nền shell | `sdk/shell/tos_env.sh` | **Đã chạy thử trên máy thật** (`selftest.sh`: detect + CPU restore + `tos_run` OK) |
| Đặc tả app | `sdk/spec/app.json` | Mẫu |
| Đóng gói 4 hệ | `tools/package_app.ps1` | Chạy trên PC, zip dùng `/`; **chưa thử giải nén/cài trên máy** |
| App mẫu shell | `apps_mau/hello_shell` | Chưa chạy từ MainUI |
| Overlay hệ thống | `rootfs_overlay/` | udev input, sysctl: **đã áp dụng trên máy** |
| Thư viện Rust (fb, evdev, audio, input map) | `sdk/rust/` | **Chưa viết**: PC chưa có Rust/Docker |

## Ma trận khác biệt đã xử lý trong `tos_env.sh`
- Phát hiện OS: NextUI (`$PLATFORM`/`MinUI.zip`) → Knulli (`/userdata/system`, `batocera-version`) → Spruce (`/mnt/SDCARD/spruce`) → Stock (`/usr/trimui/bin/runtrimui.sh`) → generic.
- Chỉ **Stock** mới STOP/CONT MainUI (MainUI tự thoát khi mở app; runtrimui tự bật lại sau khi launcher kết thúc).
- Mã thoát `42` = chạy lại (OTA).

## Việc còn lại cho Bước 4
- Âm thanh (ALSA Stock vs PulseAudio/PipeWire Knulli), Bluetooth, khôi phục âm lượng.
- Thư viện Rust: framebuffer (ioctl, stride, pan double-buffer), evdev (EVIOCGBIT, ABS_HAT), map phím theo `CLAUDE.md` dự án thẻ.
- Chạy thử gói Stock từ MainUI và các gói còn lại trên CFW thật (chưa có máy chạy Knulli/Spruce/NextUI để thử).
