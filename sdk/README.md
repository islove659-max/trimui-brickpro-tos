# SDK TOS — làm app cho TrimUI Brick Pro (Stock OS 1.1.1)

Đã kiểm chứng trên máy thật (2026-10-08): app mẫu `hello-rust` chạy 60 FPS có pan 2 trang + vsync, nhận tay cầm, phát âm thanh, thoát sạch, cài qua thẻ SD.

## Cấu trúc
| Thư mục | Nội dung |
|---|---|
| `shell/tos_env.sh` | Lớp nền launcher: phát hiện OS/máy/độ phân giải, lưu-đặt-khôi phục CPU governor, dừng/bật MainUI (chỉ Stock), `tos_run` có trap và vòng lặp mã thoát 42 |
| `rust/` | Thư viện + app mẫu Rust, **không phụ thuộc crate ngoài** (cần `rust-lld`, không cần Docker/MSVC): `fb.rs` (ioctl, stride, pan 2 trang, vsync), `input.rs` (evdev, quét thiết bị, ABS_HAT→phím), `audio.rs` (luồng PCM liên tục qua `aplay`) |
| `spec/*.json` | Manifest app (`id`, `name`, `version`, `cpu_profile`, `needs`...) |
| `../tools/package_app.ps1` | Đóng gói 1 thư mục app thành 4 zip: Stock, Knulli, SpruceOS, NextUI (tên mục dùng `/`) |

## Build và đóng gói
```
rustup target add aarch64-unknown-linux-musl     # một lần
cd sdk/rust && cargo build --release            # .cargo/config.toml đã đặt target + linker rust-lld
cd tools && .\package_app.ps1 -AppDir ..\apps_mau\hello_rust -Spec ..\sdk\spec\hello-rust.json
```
Binary là ELF aarch64 tĩnh (~430KB, `opt-level=z`, LTO, `panic=abort`).

## Bài học bắt buộc nhớ (đều đã gặp thật)
1. **Phím:** nút Home hông = keycode 172, MENU = 316, nguồn = 116 (`event1`), âm lượng 115/114 (`event3`). Tay cầm nằm ở `event3` "TRIMUI Player1". Đọc hết hàng đợi tới `EAGAIN`.
2. **Âm thanh:** KHÔNG đụng mixer. Mức do MainUI để lại (`digital volume=0`, `Headphone Volume=1`, `DAC=140`) là mức TO; thang đo NGƯỢC so với dB (đặt 63/7 làm im lặng). Phát PCM S16_LE 48000Hz 2 kênh vào `aplay -D PlaybackDmix`. Đừng ghi đoạn ngắn vào `aplay`: dùng luồng liên tục (có im lặng) rồi trộn tiếng vào. Mạch loa cần ~1 giây khởi động.
3. **Mở app trên Stock:** MainUI ghi `/tmp/cmd_to_run.sh` (`cd <app>; ./launch.sh`) rồi TỰ THOÁT; `runtrimui.sh` chạy file đó, xong chạy `premainui.sh` rồi bật lại MainUI. Khi app chạy MainUI đã thoát và thiết bị âm thanh đã được trả (`pcm=closed`).
4. **Hiện app trong danh sách Apps:** thư mục `Apps/<id>/` cần `config.json` (viết **ASCII, không dấu**), `launch.sh` (LF) và **`icon.png`** (120x120). MainUI chỉ quét lại danh sách khi khởi động ⇒ sau khi chép app mới phải mở rồi thoát một app (hoặc khởi động lại MainUI) mới thấy.
5. **zip:** `ZipFile.CreateFromDirectory` của Windows PowerShell 5.1 dùng `\` trong tên mục ⇒ hỏng trên Linux; script đã tự ghi zip với `/`. FAT không giữ quyền thực thi: gọi qua `sh`, đừng dựa vào `chmod`.
6. **OSD:** phím nóng mặc định HOME (nút hông) hoạt động ngay; MENU+SELECT chỉ hiện khi đặt trong Cài đặt → Phím nóng OSD.
7. **Thoát:** app phải thoát bằng MENU (316) hoặc B (304), bắt SIGTERM/SIGINT, dọn fb (xoá đen + pan về trang 0), `kill`+`wait` tiến trình con (aplay).
8. **Công cụ PowerShell:** bộ lọc an toàn chặn nhầm một số chuỗi (`rm` cạnh `/tmp`, `sed s///`, tên hàm `rd`...). Gửi lệnh nhiều dòng bằng script LF qua `scp`, không nhét vào chuỗi `ssh`.

## Chưa làm / chưa kiểm chứng
- Gói Knulli, SpruceOS, NextUI mới đúng cấu trúc theo tài liệu, **chưa thử trên hệ thật** (máy chỉ chạy Stock).
- Chưa có: ánh xạ phím theo cấu hình người dùng, đọc `vol`/`mute` hệ thống, phát file âm thanh (WAV/OGG), render chữ (fontdue), OTA.
