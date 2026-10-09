# TrimUI Brick Pro — TOS (firmware tối ưu) + glremote

Dự án cá nhân, không liên kết với TrimUI. Mục tiêu: làm Stock OS của **TrimUI Brick Pro (TG4040, Allwinner A133plus, PowerVR GE8300, 1024x768)** chạy được nhiều thứ hơn mà không phá hệ thống gốc.

*English summary:* tooling and a GLES/audio remoting runtime (**glremote**) that lets 32-bit ARMHF games use the real PowerVR GPU and audio on the Brick Pro stock OS, which ships no 32-bit GPU driver. A 32-bit shim (`libEGL`/`libGLESv2`/fake `libdrm`/`libgbm`/`libasound`) forwards commands through shared memory to a 64-bit Rust server. Verified on hardware with Apotris (armhf): 60 FPS with sound.

## Tải về (Releases)

| File | Dùng để |
|---|---|
| `glremote-sd-pack-*.zip` | Giải nén vào **gốc thẻ SD** → `System/bin/glremote_run` + `System/glremote/`. Cho firmware có runtime ARMHF (tos-3.0 trở lên). |
| `TrimUI PortMaster + glremote (Brick Pro) *.zip` | Gói PortMaster gốc cho TrimUI (không sửa) **kèm sẵn** glremote — cài PortMaster lần đầu là có. |

Ảnh firmware đầy đủ **không** được đăng (chứa phần mềm bản quyền của TrimUI). Công cụ trong `tools/` tự dựng ảnh *tos-4.0* từ ảnh gốc của chính bạn (`build_fw_v2.py`), xem [docs/FIRMWARE_V4.md](docs/FIRMWARE_V4.md).

## Dùng glremote

Trong script của port PortMaster chỉ có bản ARMHF, thay dòng chạy game:

```sh
export GAMELIBS="$GAMEDIR/libs.armhf"      # thư viện 32-bit của game (nếu có)
glremote_run ./TenGame.armhf               # hoặc /mnt/SDCARD/System/bin/glremote_run
```

Biến tuỳ chọn: `GLR_MAX` (giới hạn giây), `GLR_NULL=1` (thử không màn hình/loa), `SDL_VIDEODRIVER` (mặc định `KMSDRM_LEGACY`), `SDL_AUDIODRIVER` (mặc định `alsa`; cả hai là bản giả do glremote cung cấp).

Giới hạn: chỉ OpenGL ES 2 (không GLES3/VAO), một luồng GL, âm thanh S16 48 kHz, chưa ghi âm. SDL2 32-bit dự phòng (Debian 11) segfault khi `SDL_QuitSubSystem(JOYSTICK)` lúc thoát; game mang SDL2 riêng không bị.

## Cấu trúc

| Thư mục | Nội dung |
|---|---|
| `armhf_gl/client` | shim 32-bit `no_std` (Rust, syscall ARM EABI thô): `glremote`, `alsa_shim`, `drmgbm_stub`, `sdl_deps_stub`, app thử |
| `armhf_gl/server_rs` | máy chủ 64-bit `glserver` (Rust `no_std`, nạp SDL2 lúc chạy): GPU + tiến trình âm thanh (`--audio`, OSS `/dev/dsp`) + chế độ `--null` |
| `armhf_gl/build_runtime.ps1`, `pack_runtime.py`, `build_sd_pack.py` | dựng runtime và các gói phát hành |
| `tools/` | dựng ảnh firmware (ext4lite), kiểm toán rootfs, kiểm thử máy ảo QEMU, ghi thẻ SD (có chốt an toàn) |
| `docs/` | kế hoạch, kiến trúc, từng bản firmware (v2/v3/v4), thiết kế glremote ([ARMHF_DOHOA.md](docs/ARMHF_DOHOA.md)) |
| `rootfs_overlay/`, `sdk/`, `apps_mau/` | file chồng lên rootfs, SDK Rust (fb/evdev/audio/chữ Việt), app mẫu |

## Dựng lại

Cần Rust (target `aarch64-unknown-linux-musl`, `armv7-unknown-linux-gnueabihf`, linker `rust-lld`, không cần MSVC). `powershell -File armhf_gl/build_runtime.ps1` rồi `python armhf_gl/build_sd_pack.py`. Firmware: `python tools/build_fw_v2.py --armhf <thư_mục> --glremote armhf_gl/dist` (cần ảnh recovery gốc và gói ARMHF Debian 11 trích bằng `tools/lay_armhf.py`).

## Kiểm thử

Bộ kiểm thử máy ảo QEMU aarch64 chạy trên rootfs thật: `tools/vm_guest_tests.sh` (38 kiểm tra chung), `tools/vm_guest_glremote.sh` (33), `tools/vm_guest_glremote_sd.sh` (16). Máy ảo không có GPU/loa nên glremote chạy chế độ `--null`; phần phần cứng thật được kiểm trên máy.

## Cảnh báo

Nạp firmware hay ghi đĩa có thể làm mất dữ liệu; dùng thẻ riêng, xem chốt an toàn trong `tools/ghi_the_sd_v2.ps1`. Phần mềm cung cấp "nguyên trạng", tự chịu rủi ro.

## Giấy phép và ghi công

Mã nguồn của dự án: MIT ([LICENSE](LICENSE)). Thành phần bên thứ ba: xem [NOTICE](NOTICE) (PortMaster — MIT; SDL2 Debian — zlib; gói PortMaster gốc không bị sửa).
