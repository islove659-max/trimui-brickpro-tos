# Đồ hoạ ARMHF (32-bit) trên Brick Pro — khảo sát 2026-10-09

## Nền đã có (firmware tos-3.0)
glibc 2.31 / libgcc_s / libstdc++ 32-bit + trình nạp `/lib/ld-linux-armhf.so.3`. Đã xác nhận trên máy thật: chạy `libc.so.6` 32-bit và `7zzs.armhf` của PortMaster.

## Vấn đề đồ hoạ (đã đo, không phải suy đoán)
- **Máy KHÔNG có driver GPU 32-bit.** `/usr/lib/libEGL*, libGLESv2*, libGLES_CM*, libIMGegl*, libsrv_um*, libusc*` đều là **aarch64** (ELF machine 183). Quét thêm các cây firmware trên PC (Smart Pro base, Brick base, NXRedux, TRIMUI_EX, bản sao lưu): chỉ có 4 thư viện đồ hoạ, **cả 4 đều aarch64**; không có bản ARM 32-bit nào của driver PowerVR GE8300.
- **Không có SDL2 32-bit trên máy.** Mọi port ARMHF phải tự mang libSDL2 32-bit (`gptokeyb.armhf` báo `libSDL2-2.0.so.0: wrong ELF class: ELFCLASS64` khi chỉ thấy bản 64-bit).
- **`weston_pkg_0.2` (PortMaster)** có lớp "crusty" 32-bit (`lib_armhf/graphics/crusty_gbm|crusty_x11egl|crusty_glx`, `gl4es_glxpass`): chúng giả `libEGL/libgbm/libdrm/libudev` cho app 32-bit rồi chuyển tới Weston 64-bit. Nhưng phần **vẽ GLES** vẫn cần `libGLESv2` 32-bit của HĐH (qua `CRUSTY_LIBEGL32` / `findlib32`). Chính `westonwrap.sh` ghi: chế độ Mesa llvmpipe và virgl "chỉ hỗ trợ aarch64"; `mesa_pkg_0.1` chỉ có bản aarch64 (kèm `virgl_test_server`, `libvirglrenderer`, LLVM 19).
- Kết luận: với thành phần sẵn có, **không thể** có GLES tăng tốc phần cứng cho app 32-bit trên máy này.

## Hướng phát triển khả dĩ
| Hướng | Mô tả | Ưu | Nhược |
|---|---|---|---|
| A. Phần mềm (Mesa llvmpipe armhf) | tải Debian armhf Mesa + LLVM (~60MB) | không tự viết | chậm trên A53 (vài FPS–chục FPS), nặng, PortMaster nói chưa hỗ trợ armhf |
| **B. GLES remoting tự viết** | `libEGL.so.1` + `libGLESv2.so.2` **32-bit** (shim) tuần tự hoá lệnh qua bộ nhớ chung tới **máy chủ 64-bit** chạy GPU PowerVR thật | tăng tốc phần cứng; hợp game 2D GLES2 (GameMaker/Unity 2D/SDL2) | công lớn: ~25 hàm EGL + ~140 hàm GLES2, truyền texture/buffer, đồng bộ present; cần biên dịch chéo armv7 + máy chủ aarch64 glibc |
| C. Lấy driver 32-bit từ nhà sản xuất | tìm gói IMG/Allwinner A133 có `lib32` | rẻ nhất nếu có | hiện không tìm thấy nguồn |

## Đề xuất
Hướng B theo từng chặng, mỗi chặng kiểm trên máy thật:
1. Chương trình thử 32-bit (armv7 gnueabihf) in thông tin + chạy trên máy (xác nhận chuỗi công cụ biên dịch chéo).
2. Máy chủ 64-bit (aarch64 glibc) tạo ngữ cảnh EGL/GLES thật và vẽ một khung hình ra màn hình.
3. Giao thức + shim: `eglGetDisplay/Initialize/CreateContext/CreateWindowSurface/MakeCurrent/SwapBuffers` và nhóm GLES2 cơ bản (shader, buffer, texture, draw, uniform).
4. Ứng dụng thử 32-bit vẽ tam giác + texture ở 60 FPS.
5. Mở rộng hàm cho đến khi một game 32-bit thật chạy được.
Cần tải thêm: mục tiêu Rust `armv7-unknown-linux-gnueabihf` (~vài chục MB) qua rustup.

---
# Kết quả Hướng B (2026-10-09) — GLES remoting CHẠY được trên máy thật
- Chặng 1: máy chủ SDL2+GLES2 64-bit trên PowerVR GE8300 (GLES 3.2) đạt 60 FPS (`armhf_gl/server/gl_poc.py`).
- Chặng 2: chuỗi công cụ armv7 không libc (rust-lld, `armhf_gl/client/.cargo/config.toml`) chạy qua `/lib/ld-linux-armhf.so.3` OK.
- Chặng 3–4: `armhf_gl/client/glremote` = shim 32-bit (`libglremote.so`, 31KB, no_std, syscall ARM EABI thô) xuất ~25 hàm EGL + ~110 hàm GLES2; đóng gói lệnh vào bộ nhớ chung `/tmp/glremote.shm` (vòng đệm 4MiB, phản hồi đồng bộ cho Gen*/Get*/Swap). `armhf_gl/server/gl_server.py` giải mã và thực thi bằng GPU thật. Mảng đỉnh phía client được đẩy tự động trước lệnh vẽ; texture/buffer chuyển không sao chép thừa (con trỏ thẳng vào vùng chung).
- **Ứng dụng thử 32-bit `gl_test`** (EGL init, shader, texture 16x16, client-array, xoay): `GL_RENDERER: PowerVR Rogue GE8300`, **361 khung/6,0s = 59,5 FPS** (vsync), ~540 lệnh/s mỗi khung nhẹ. Chạy bằng `armhf_gl/run.sh` (ví dụ qua `/tmp/cmd_to_run.sh` + `kill -9 MainUI`).
- Build: `cargo build --release -p gl_test` và `cargo rustc --release -p glremote -- -C link-arg=--version-script=glremote/exports.map` (map ẩn memcpy/memset… để không ghi đè libc của game).
- Hạn chế đã biết: `glGetError` luôn trả 0 (lỗi thật ghi vào log máy chủ); máy chủ Python giới hạn ~1000 lệnh/khung ở 55 FPS (cần máy chủ native nếu game nặng); `glDrawElements` với VBO phần tử + mảng đỉnh client chưa hỗ trợ; một luồng GL; mới chỉ có GLES2 lõi (chưa VAO/GLES3, chưa extension).
- Việc tiếp: đặt tên bản sao `libEGL.so.1`/`libGLESv2.so.2` cho game thật, SDL2 32-bit chế độ offscreen/KMSDRM giả, máy chủ native (Rust aarch64), gói runtime vào SD/firmware v4, thử game 32-bit PortMaster.

## Máy chủ Rust (2026-10-09) — thay gl_server.py
`armhf_gl/server_rs/glserver` (aarch64, `no_std`, syscall thô, 35KB; nhập SDL2 động lúc chạy qua khung `sdl_stub` chỉ để liên kết; build bằng `server_rs/build.ps1`, không cần MSVC/Docker). Cùng giao thức với shim `glremote`. Thử trên máy thật với `gl_test` 32-bit: 361 khung/6,05s ≈ 59,6 FPS, 3.275 lệnh, hết phụ thuộc Python. Mảng đỉnh client và chỉ số được trỏ thẳng vào vòng đệm (không sao chép). Chạy: `armhf_gl/run_rs.sh` (đặt glserver, gl_test, libglremote.so vào /tmp/armhf_ctest). Tham số `--seconds N` = thời gian rảnh tối đa.

## Thử với SDL2 32-bit thật (2026-10-09)
- Đã tải (có đồng ý, SHA256 khớp Packages) `libsdl2-2.0-0_2.0.14+dfsg2-3+deb11u1_armhf.deb`. Thư viện nạp được trên máy thật khi các phụ thuộc (X11, wayland, pulse, drm, gbm, xkbcommon, asound) được thay bằng khung rỗng có đủ ký hiệu + phiên bản (`client/sdl_deps_stub`, `xkb.map/pulse.map/alsa.map`). Lỗi ld.so "check_match" là do thiếu verdef trong khung.
- shim `glremote` đã dùng chung vị trí ghi/bộ đếm phản hồi qua tiêu đề bộ nhớ chung nên `libEGL.so.1` và `libGLESv2.so.2` (hai bản sao) chạy cùng nhau được.
- **Chặn:** SDL2 của Debian chỉ có driver x11/wayland/KMSDRM/dummy, KHÔNG có `offscreen` hay `mali`, nên không tạo được cửa sổ OpenGL ES nếu không giả cả libdrm/libgbm (kiểu "crusty" của PortMaster). SDL2 đi kèm port thật thường có driver `mali`/`kmsdrm`/`fbdev`; shim đã chấp nhận mọi native window nên khớp driver kiểu `mali`. Cần một port ARMHF thật để tùy chỉnh.
- `glremote_run.sh` nay có bộ đếm giờ `GLR_MAX` (kill -9 server + app khi quá hạn) để tránh treo màn đen (đã gặp một lần khi app lỗi nạp mà server vẫn chạy).

## SDL2 32-bit chạy qua glremote (2026-10-09) — Apotris ARMHF
- Thêm `drmgbm_stub` (`libdrm.so.2` + `libgbm.so.1` giả: 1 màn hình 1024x768, CRTC/connector/encoder ảo, gbm bo/surface giả, `drmModePageFlip` + `drmHandleEvent` gọi lại handler) để SDL2 Debian dùng driver `KMSDRM_LEGACY`. Cần tệp `/dev/dri/card0` (script tạo tệp rỗng rồi xoá; máy có sẵn `/dev/dri/renderD128`).
- shim: `eglGetConfigAttrib(NATIVE_VISUAL_ID)=GBM XRGB8888`; `eglBindAPI` từ chối OpenGL (chỉ ES) để SDL_Renderer bỏ qua driver `opengl` rồi chọn `opengles2`; thêm `glShaderBinary` (SDL bắt buộc có); bản sao `libGL.so.1`/`libOpenGL.so.0` (SDL nạp cả khi cửa sổ không đòi GL).
- Kết quả trên máy thật: `sdl_test` (cửa sổ OPENGL) 60 FPS; `sdl_rtest` (SDL_CreateRenderer như Apotris) 60 FPS; **Apotris.armhf chạy 60 khung/s liên tục** (~19 lệnh GL/khung). Chưa có âm thanh (SDL dùng driver dummy) và chưa có điều khiển (gptokeyb armhf).
- Chạy an toàn khi thử: `killall -STOP MainUI` + bộ đếm `(sleep N; killall -CONT MainUI; killall -9 glserver ld-linux-armhf.so.3) &` — không dùng kill MainUI vì dễ kẹt màn đen.
- Việc còn lại: âm thanh (ALSA giả → chuyển PCM sang máy chủ 64-bit), tay cầm (evdev qua SDL KMSDRM đọc được /dev/input; gptokeyb), đóng gói runtime vào `Apps/` hoặc firmware v4, tích hợp preflight PortMaster cho port armhf, crash khi SDL_Quit (segfault trong sdl_rtest).

## Âm thanh (2026-10-09)
- `alsa_shim` = `libasound.so.2` giả (32-bit, có version ALSA_0.9/rc4): chỉ nhận S16_LE, 48000 Hz, 1–2 kênh (SDL tự đổi mẫu); `snd_pcm_writei` chép PCM vào vòng 256KiB trong `/tmp/glremote.snd` và chặn khi đầy để giữ nhịp.
- `glserver --audio` (tiến trình riêng, 64-bit) đọc vòng và phát ra `/dev/dsp` (OSS, 4 mảnh 2KiB). `glremote_run.sh` chạy cả ba (server GL, server audio, app), bật loa (`/sys/class/speaker/mute`=0), có `timeout -s 9` và bộ đếm giờ.
- Lưu ý: `/dev/snd/pcmC0D0p` bị MainUI giữ (kể cả khi bị STOP) → âm thanh chỉ mở được khi MainUI đã thoát (đúng luồng chạy app thật). Test hình-only dùng được STOP; test có tiếng phải để MainUI thoát (cmd_to_run).
- **Kết quả: Apotris ARMHF có hình 60 FPS + tiếng to rõ + chơi được trên Brick Pro.** (người dùng xác nhận)
- Còn: đóng gói thành runtime/launcher cho port ARMHF (PortMaster `DEVICE_ARCH`), preflight tự dùng glremote khi game armhf, firmware v4 chứa runtime, lỗi segfault lúc SDL_Quit, tay cầm qua gptokeyb armhf nếu port cần.
