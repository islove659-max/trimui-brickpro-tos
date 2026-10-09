# Firmware v4 (tos-4.0) = v3 + glremote (GPU + am thanh cho game ARMHF 32-bit) - 2026-10-09

Anh: `out/fw_v4/sd_recovery_tg4040_brickpro_ver1.1.1_20260717_tos4.img` (SHA256 trong file .sha256).
Build: `powershell -File armhf_gl/build_runtime.ps1` roi
`python -I -X utf8 tools/build_fw_v2.py --armhf C:\Users\X\tools\armhf_x --glremote armhf_gl\dist`.

## Them vao rootfs (so voi v3): 10 file + 22 symlink + 3 thu muc, 1,1 MB
- `/usr/bin/glremote_run`  - lenh chay game 32-bit (dung trong script PortMaster).
- `/usr/lib/glremote/glserver` (aarch64) - may chu GPU (`glserver --audio` = tien trinh am thanh).
- `/usr/lib/glremote/lib32/` - libEGL/libGLESv2/libGL (shim, symlink ve libglremote.so), libdrm/libgbm gia, libasound gia, khung X11/Wayland/Pulse/xkb.
- `/usr/lib/glremote/lib32-sdl2/libSDL2-2.0.so.0` - SDL2 32-bit du phong (Debian 11, 2.0.14); game mang SDL2 rieng thi uu tien cua game (`GAMELIBS`).
- `/usr/lib/glremote/README.txt`.
Khong sua bat ky file goc nao ngoai 2 hook cua v2 (kiem toan: MAT: khong, MOI: 75, DOI: 2).

## Kiem thu truoc khi nap (may ao QEMU aarch64, rootfs that cua ban v4)
- Bo kiem thu chung (`vm_guest_tests.sh`): **38/38 dat** (khong hoi quy v2/v3).
- Bo kiem thu glremote (`vm_guest_glremote.sh`): **33/33 dat** - ung dung 32-bit `selftest` chay THAT qua toan chuoi: SDL2 32-bit -> KMSDRM_LEGACY (libdrm/libgbm gia) -> shim EGL/GLES -> bo nho chung -> `glserver --null`; am thanh SDL -> libasound gia -> `glserver --audio`; dem 150 khung, hon 1200 lenh GL, callback am thanh; don dep /dev/dri/card0, /tmp/glremote.*; app loi thoat gon.
- May ao KHONG co GPU/man hinh/loa: che do `--null` bo qua SDL va GL that. Phan GPU/loa that da kiem tren may that (Apotris armhf 60 FPS co tieng, ban thu nghiem truoc khi dong goi).
- Loi da biet (SDL2 Debian du phong): thoat bang SDL_QuitSubSystem(JOYSTICK) bi segfault (sau khi video/audio da tat sach). Game mang SDL2 rieng khong bi.

## Khi nap
Khong doi bootloader/nhan/boot-resource/recovery (da so tung byte voi anh goc: 0 khoi 4KB khac ngoai vung rootfs/env). Nap vao the SD can quyen quan tri va xoa the (anh recovery ghi de eMMC) - lam theo `docs/KET_QUA_SAU_FLASH.md`; sao luu the/thiet bi truoc.

## Goi cho the SD / PortMaster (glremote 0.1.0) - cung ngay
- `out/pkg/glremote-sd-pack-0.1.0.zip` (0,77 MB): giai nen vao goc the SD => `System/bin/glremote_run` + `System/glremote/` (ban sao thay symlink vi FAT32). Dung cho tos-3.0 hoac cap nhat rieng; firmware tos-4.0 uu tien `/usr/lib/glremote`.
- `out/pkg/TrimUI PortMaster + glremote (Brick Pro) 0.1.0.zip` (25 MB): goi PortMaster goc cho TrimUI (`Downloads/trimui.portmaster.zip`, khong sua file nao) + glremote => cai PortMaster lan dau la co san glremote (kem `HUONG_DAN_GLREMOTE.txt`).
- Kiem thu may ao tren rootfs tos-3.0 (KHONG co glremote trong firmware): `tools/vm_guest_glremote_sd.sh` 16/16 dat (goi SD giai nen vao /mnt/SDCARD, chay selftest 32-bit that, don dep).
- PortMaster (`hardware.py`/`device_info.txt`) tu nhan ARMHF khi co `/lib/ld-linux-armhf.so.3` (capabilities armhf+aarch64, DEVICE_ARCH van aarch64): port chi co ban ARMHF se duoc phep cai; script cua port goi `glremote_run ./Game.armhf`.
- Chua dang len GitHub (viec dang cong khai can nguoi dung quyet dinh).
## Nạp thẻ + đóng gói phân phối (2026-10-09 09:17)
- Đã ghi tos4.img vào thẻ 128GB (Disk 2) bằng `tools/ghi_the_sd_v2.ps1` (UAC do người dùng chấp nhận); đọc lại SHA256 thẻ = ảnh (`708d456b…`). Log: `C:\Users\X\tools\ghi_the_sd_v4.log`. **Chưa cắm vào máy để nạp eMMC** (người dùng làm).
- File phân phối: `out/dist_fw/TrimUI-BrickPro-TOS-4.0-flash-image.zip` (244 MB, SHA256 trong `.sha256`): chứa ảnh `.img` (raw, dùng balenaEtcher/Rufus DD/Win32DiskImager), `.sha256` và `HUONG_DAN_NAP.txt`. Đã giải nén thử: SHA256 ảnh khớp.
- Sau khi nạp eMMC: cài đặt MainUI (âm lượng, ngôn ngữ, nút Home) và khoá SSH trên máy bị xoá (như lần v2/v3); cần cài lại khoá SSH.
