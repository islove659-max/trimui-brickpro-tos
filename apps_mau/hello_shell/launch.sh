#!/bin/sh
# Launcher mẫu dùng lớp nền TOS. Chạy một "app" shell in thông tin môi trường ra log.
DIR=$(cd "$(dirname "$0")" && pwd)
. "$DIR/lib/tos_env.sh"
export TOS_LOG="$DIR/hello.log"
tos_detect
tos_run balanced sh -c 'echo "Hello TOS: os=$TOS_OS device=$TOS_DEVICE fb=${TOS_W}x${TOS_H}"; sleep 1'
exit $?
