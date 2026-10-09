#!/bin/sh
set -eu
restore_stock() {
  systemctl stop ankimarkable-hashtab.service 2>/dev/null || true
  /home/root/xovi/stock >/dev/null 2>&1 || true
  systemctl start xochitl
}
trap restore_stock EXIT
mkdir -p /tmp/ankimarkable-hashtab/extensions.d /home/root/xovi/exthome/qt-resource-rebuilder
ln -sf /home/root/xovi/extensions.d/qt-resource-rebuilder.so /tmp/ankimarkable-hashtab/extensions.d/qt-resource-rebuilder.so
rm -f /home/root/xovi/exthome/qt-resource-rebuilder/hashtab
systemctl stop xochitl
systemd-run --quiet --no-block --collect --unit ankimarkable-hashtab \
  --setenv XOVI_ROOT=/tmp/ankimarkable-hashtab \
  --setenv LD_PRELOAD=/home/root/xovi/xovi.so \
  --setenv QML_DISABLE_DISK_CACHE=1 \
  --setenv QMLDIFF_HASHTAB_CREATE=/home/root/xovi/exthome/qt-resource-rebuilder/hashtab \
  /usr/bin/xochitl
ready=false
for i in $(seq 1 120); do
  if test -s /home/root/xovi/exthome/qt-resource-rebuilder/hashtab; then ready=true; break; fi
  sleep 1
done
systemctl stop ankimarkable-hashtab.service || true
test "$ready" = true || { echo 'Resource map failed; stock interface restored.' >&2; exit 1; }
systemd-run --quiet --no-block --collect --unit ankimarkable-launcher-start /home/root/xovi/start
for i in $(seq 1 60); do
  test ! -S /tmp/qtfb.sock || break
  sleep 1
done
systemctl is-active --quiet xochitl
test -S /tmp/qtfb.sock || { /home/root/xovi/stock; echo 'Launcher failed; stock interface restored.' >&2; exit 1; }
echo 'AppLoad installed and running. Reboot returns to stock until autostart is configured.'
trap - EXIT
