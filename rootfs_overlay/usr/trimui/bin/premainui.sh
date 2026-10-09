#!/bin/sh

rm -f /tmp/trimui_inputd/input_no_dpad
rm -f /tmp/trimui_inputd/input_dpad_to_joystick

# [tos] trước khi MainUI chạy lại: bật lại dịch vụ đã dừng trong chế độ game
[ -x /usr/trimui/bin/tos_gamemode.sh ] && /usr/trimui/bin/tos_gamemode.sh off
