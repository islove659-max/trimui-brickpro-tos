#!/bin/sh
export LD_LIBRARY_PATH=/usr/trimui/lib:/mnt/SDCARD/System/lib
cd /tmp/armhf_ctest
./glserver --seconds 15 > /mnt/UDISK/glremote_server.out 2>&1 &
sleep 3
unset LD_LIBRARY_PATH
/lib/ld-linux-armhf.so.3 /tmp/armhf_ctest/gl_test > /mnt/UDISK/glremote_client.log 2>&1
wait
