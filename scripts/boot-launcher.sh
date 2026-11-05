#!/bin/sh
set -eu
# Firmware updates must start the stock interface until this build is retested.
test "$(hostname)" = imx93-tatsu || exit 0
grep -Eq '^IMG_VERSION="(3\.28\.0\.172|3\.29\.1\.0)"$' /etc/os-release || exit 0
test -x /home/root/xovi/start -a -x /home/root/xovi/stock || exit 0
test -s /home/root/xovi/exthome/qt-resource-rebuilder/hashtab || exit 0
if test -S /tmp/qtfb.sock; then exit 0; fi
/home/root/xovi/start
for attempt in $(seq 1 60); do
    if test -S /tmp/qtfb.sock && systemctl is-active --quiet xochitl; then exit 0; fi
    sleep 1
done
/home/root/xovi/stock
echo 'AppLoad did not start; restored the stock interface.' >&2
exit 1
