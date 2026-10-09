# Bản vá menu Music Chip n Dale

D-pad lên/xuống chạy vòng qua các mục và Music; trái/phải hoặc A đổi nhạc chỉ khi Music được chọn. B không tự đổi nhạc. Người dùng đã xác nhận menu chạy vòng hoạt động trên máy thật.

Không kèm dữ liệu game. Dùng bản game.droid gốc của bạn có SHA256 9ab576aba53742c86135d7174e016c9905298007fde3e608156911fb73676ede và UndertaleModTool CLI 0.9.2.0:

    powershell -File rebuild-music.ps1 -Cli PATH/UndertaleModCli.exe -Original PATH/game.droid -Output PATH/patched/game.droid

Script kiểm tra hash gốc trước khi biên dịch ba GML. SHA256 đầu ra đã thử: 401d751ae97ed0f659327a13f73789a923865653043a9f31630e6dbfc0d4ae75.
Sao lưu dữ liệu gốc trước khi cài. Nếu dùng GMLOADER_SAVEDIR khác, phải có cả thư mục files và splash.png.
chip-console.gptk dùng cùng trimui-controllerdb.txt của runtime. Menu đã kiểm chứng; ABXY trong màn chơi còn cần thử.
