# TOS 4.0 — cập nhật runtime ngày 09/10/2026

Giữ nguyên tên/version các gói; nội dung và SHA256 thay đổi.

- Sửa framebuffer depth/stencil PowerVR: fallback packed depth24/stencil8 cho cặp renderbuffer không tương thích; giữ vòng đời và phục hồi khi thất bại.
- Sửa client vertex arrays: sao chép vào VBO thay vì tham chiếu vùng command ring có thể bị tái sử dụng.
- Bổ sung libudev ARMHF, thư viện GameMaker và database nút vật lý TrimUI. VID/PID Xbox360 của hệ điều hành vẫn giữ.
- glremote_run thực thi ELF game trực tiếp để gptokeyb tìm đúng tên tiến trình.
- Mã vá menu Music Chip n Dale ở armhf_gl/game_patches/chipndale; firmware không chứa game hoặc tự sửa dữ liệu game. Người dùng xác nhận menu chạy vòng OK.

Máy thật: Estigma hiển thị đúng ~30 FPS trong thử ngắn; menu Chip n Dale ~60 FPS qua lượt 120 giây. Chưa xác nhận mọi màn chơi, ABXY trong gameplay hoặc mọi game ARMHF. 2048 là aarch64 và chưa thử phiên này.

Ảnh dựng từ firmware gốc; checksum IMAGEWTY/env, đọc lại runtime và so sánh ngoài vùng rootfs/env đều đạt. QEMU gói mới: hệ thống 38/38, runtime ARMHF 33/33, cài trên Stock 27/27, cập nhật TOS 24/24, gói SD 16/16; tất cả fail=0. Xem VERIFICATION_20261009.txt. ZIP giải nén kiểm tra và hash payload/ảnh đều khớp.
