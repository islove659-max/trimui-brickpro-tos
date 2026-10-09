#!/bin/sh
# Chay gl_server (64-bit) + gl_test (32-bit) tren may. Dat cac file vao /tmp/armhf_ctest truoc.
export LD_LIBRARY_PATH=/usr/trimui/lib:/mnt/SDCARD/System/lib PYTHONHOME=/mnt/SDCARD/System
cd /tmp/armhf_ctest
/mnt/SDCARD/System/bin/python3 gl_server.py --log /mnt/UDISK/glremote_server.log --seconds 15 > /mnt/UDISK/glremote_server.out 2>&1 &
sleep 4
unset LD_LIBRARY_PATH
/lib/ld-linux-armhf.so.3 /tmp/armhf_ctest/gl_test > /mnt/UDISK/glremote_client.log 2>&1
wait