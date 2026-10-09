# TrimUI Brick Pro — TOS (firmware tối ưu) + glremote

Dự án cá nhân, không liên kết với TrimUI. Mục tiêu: làm Stock OS của **TrimUI Brick Pro (TG4040, Allwinner A133plus, PowerVR GE8300, 1024x768)** chạy được nhiều thứ hơn mà không phá hệ thống gốc.

*English summary:* tooling and a GLES/audio remoting runtime (**glremote**) that lets 32-bit ARMHF games use the real PowerVR GPU and audio on the Brick Pro stock OS, which ships no 32-bit GPU driver. A 32-bit shim (`libEGL`/`libGLESv2`/fake `libdrm`/`libgbm`/`libasound`) forwards commands through shared memory to a 64-bit Rust server. Verified on hardware with Apotris (armhf): 60 FPS with sound.

## Tải về và cài (Releases)

Có **2 cách**: (A) nạp nguyên firmware tos-4.0 (đủ mọi thứ, 1 file), hoặc (B) bộ cài chạy trên máy (không xoá dữ liệu).

### A. Firmware nạp một phát — `TOS-4.0-Flash-Firmware-BrickPro.zip` (khoảng 255 MiB)
Giải nén ra ảnh `.img`, ghi vào **thẻ SD trống** bằng balenaEtcher / Rufus (DD) / Win32 Disk Imager, cắm thẻ vào Brick Pro và bật nguồn: máy tự nạp lại bộ nhớ trong. Đã gồm runtime 32-bit, glremote, chế độ game, nhận thiết bị, tinh chỉnh bộ nhớ và `loglevel=4`. **Xoá cài đặt trong máy (âm lượng, ngôn ngữ…) và khoá SSH; dùng thẻ riêng, không dùng thẻ game.** Chỉ dành cho Brick Pro firmware gốc 1.1.1 (20260717). Hướng dẫn nằm trong zip (`HUONG_DAN_NAP.txt`). PortMaster nằm trên thẻ game nên vẫn cài riêng (gói `TOS-Installer+PortMaster`).

### B. Bộ cài trên máy (không nạp firmware, không xoá dữ liệu)

| File | Dùng để |
|---|---|
| `TOS-Installer+PortMaster-4.0.zip` | **Thẻ mới / chưa có PortMaster:** bộ cài + gói PortMaster gốc cho TrimUI (không sửa). |
| `TOS-Installer-4.0.zip` | **Thẻ đã có PortMaster:** chỉ bộ cài (không ghi đè PortMaster của bạn). |

1. Giải nén zip vào **gốc thẻ SD** (cắm thẻ vào máy tính; thư mục `Apps`, `System` sẽ được gộp).
2. Cắm thẻ vào Brick Pro, vào mục Apps, mở **"TOS - Cai dat / Cap nhat"**. Màn hình báo tiến trình; xong thì khởi động lại máy.
3. Gỡ bất cứ lúc nào bằng app **"TOS - Go cai dat"**. Chi tiết và cơ chế an toàn: [docs/INSTALLER.md](docs/INSTALLER.md).

Bộ cài thêm: runtime ARMHF 32-bit, glremote (game 32-bit chạy bằng GPU thật + âm thanh), luật nhận thiết bị đầu vào, tinh chỉnh bộ nhớ, và "chế độ game" (tự tạm dừng dịch vụ nền khi vào game).

> **Bản quyền và quyền thu hồi:** ảnh firmware (cách A) là bản gốc 1.1.1 của TrimUI cộng các thay đổi của dự án này (bootloader, nhân, recovery giữ nguyên từng byte). Phần mềm gốc **thuộc quyền sở hữu của nhà sản xuất TrimUI; nhà sản xuất có toàn quyền yêu cầu thu hồi/gỡ file này bất cứ lúc nào**, và người đăng cam kết gỡ ngay khi nhận được yêu cầu. File chỉ được đăng để tiện cho người dùng máy cũ đã ngừng cập nhật, không nhằm mục đích thương mại. Không có yêu cầu nào thì file vẫn nằm ở đây; ai muốn chắc chắn có thể tự dựng từ ảnh gốc của chính mình bằng `tools/build_fw_v2.py`, xem [docs/FIRMWARE_V4.md](docs/FIRMWARE_V4.md).
## Dùng glremote

Trong script của port PortMaster chỉ có bản ARMHF, thay dòng chạy game:

```sh
export GAMELIBS="$GAMEDIR/libs.armhf"      # thư viện 32-bit của game (nếu có)
glremote_run ./TenGame.armhf               # lệnh có sẵn sau khi cài bộ cài TOS
```

Biến tuỳ chọn: `GLR_MAX` (giới hạn giây), `GLR_NULL=1` (thử không màn hình/loa), `SDL_VIDEODRIVER` (mặc định `KMSDRM_LEGACY`), `SDL_AUDIODRIVER` (mặc định `alsa`; cả hai là bản giả do glremote cung cấp).

Giới hạn: chỉ OpenGL ES 2 (không GLES3/VAO), một luồng GL, âm thanh S16 48 kHz, chưa ghi âm. SDL2 32-bit dự phòng (Debian 11) segfault khi `SDL_QuitSubSystem(JOYSTICK)` lúc thoát; game mang SDL2 riêng không bị.

## Cấu trúc

| Thư mục | Nội dung |
|---|---|
| `armhf_gl/client` | shim 32-bit `no_std` (Rust, syscall ARM EABI thô): `glremote`, `alsa_shim`, `drmgbm_stub`, `sdl_deps_stub`, app thử |
| `armhf_gl/server_rs` | máy chủ 64-bit `glserver` (Rust `no_std`, nạp SDL2 lúc chạy): GPU + tiến trình âm thanh (`--audio`, OSS `/dev/dsp`) + chế độ `--null` |
| `armhf_gl/build_runtime.ps1`, `pack_runtime.py` | dựng runtime glremote |
| `installer/` | bộ cài TOS trên máy (`install.sh`) và script dựng gói phát hành (`build_installer.py`) |
| `tools/` | dựng ảnh firmware (ext4lite), kiểm toán rootfs, kiểm thử máy ảo QEMU, ghi thẻ SD (có chốt an toàn) |
| `docs/` | kế hoạch, kiến trúc, từng bản firmware (v2/v3/v4), thiết kế glremote ([ARMHF_DOHOA.md](docs/ARMHF_DOHOA.md)) |
| `rootfs_overlay/`, `sdk/`, `apps_mau/` | file chồng lên rootfs, SDK Rust (fb/evdev/audio/chữ Việt), app mẫu |

## Dựng lại

Cần Rust (target `aarch64-unknown-linux-musl`, `armv7-unknown-linux-gnueabihf`, linker `rust-lld`, không cần MSVC). `powershell -File armhf_gl/build_runtime.ps1` rồi `python installer/build_installer.py`. Firmware: `python tools/build_fw_v2.py --armhf <thư_mục> --glremote armhf_gl/dist` (cần ảnh recovery gốc và gói ARMHF Debian 11 trích bằng `tools/lay_armhf.py`).

## Kiểm thử

Bộ kiểm thử máy ảo QEMU aarch64 chạy trên rootfs thật: `tools/vm_guest_tests.sh` (38 kiểm tra chung), `tools/vm_guest_glremote.sh` (33), `tools/vm_guest_glremote_sd.sh` (16), `tools/vm_guest_installer.sh` (27 trên Stock gốc, 24 trên rootfs đã có TOS). Máy ảo không có GPU/loa nên glremote chạy chế độ `--null`; phần phần cứng thật được kiểm trên máy.

## Cảnh báo

Nạp firmware hay ghi đĩa có thể làm mất dữ liệu; dùng thẻ riêng, xem chốt an toàn trong `tools/ghi_the_sd_v2.ps1`. Phần mềm cung cấp "nguyên trạng", tự chịu rủi ro.

## Giấy phép và ghi công

Mã nguồn của dự án: MIT ([LICENSE](LICENSE)). Thành phần bên thứ ba: xem [NOTICE](NOTICE) (PortMaster — MIT; SDL2 Debian — zlib; gói PortMaster gốc không bị sửa).

Cập nhật runtime 09/10/2026: [chi tiết và phạm vi kiểm chứng](docs/RELEASE_UPDATE_20261009.md). Tên gói giữ nguyên; dùng SHA256 mới trên Release.
