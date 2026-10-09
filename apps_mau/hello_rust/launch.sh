#!/bin/sh
# Launcher app mẫu Rust (TOS). Thoát: MENU hoặc B. A = bíp, X = phát demo.wav. Log: hello_rust.log cạnh file này.
DIR=$(cd "$(dirname "$0")" && pwd)
. "$DIR/lib/tos_env.sh"
export TOS_LOG="$DIR/hello_rust.log"
tos_detect
chmod +x "$DIR/bin/hello-tos" 2>/dev/null   # FAT không giữ quyền thực thi; vô hại nếu không có tác dụng
ARGS="--audio --wav $DIR/demo.wav"
[ -f "$DIR/controls.cfg" ] && ARGS="$ARGS --keymap $DIR/controls.cfg"
tos_run balanced "$DIR/bin/hello-tos" $ARGS
exit $?