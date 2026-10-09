glremote — GPU + am thanh cho game ARMHF 32-bit tren TrimUI Brick Pro (TrimUI OS tos-4.0)
=========================================================================================
May khong co driver GPU 32-bit. glremote giai quyet bang cach:
  - libEGL.so.1 / libGLESv2.so.2 / libGL.so.1 (32-bit, gia) ghi lenh GLES2 vao bo nho chung /tmp/glremote.shm;
  - /usr/lib/glremote/glserver (64-bit) doc lenh va chay tren GPU PowerVR that;
  - libdrm/libgbm gia cho SDL2 chon driver KMSDRM_LEGACY; libasound.so.2 gia -> glserver --audio -> /dev/dsp.

Dung:   glremote_run ./Game.armhf            (trong thu muc game; GAMELIBS=thu muc libs 32-bit cua game)
Chay thu khong man hinh:   GLR_NULL=1 GLR_MAX=10 glremote_run ./Game.armhf
Thu vien SDL2 32-bit di kem (lib32-sdl2/) chi la du phong: game mang SDL2 rieng thi uu tien cua game.
Gioi han: chi GLES2 (khong GLES3/VAO), mot luong GL, am thanh S16 48 kHz, chua co ghi am.
Nguon: du an "xay lai OS trimui", thu muc armhf_gl/.
