#!/usr/bin/env bash
set -euo pipefail
host=${1:-root@10.11.99.1}
ssh_options=(-o ConnectTimeout=10)
if [[ -n ${SSH_CONTROL_PATH:-} ]]; then ssh_options+=(-S "$SSH_CONTROL_PATH"); fi
ssh "${ssh_options[@]}" "$host" sh <<'REMOTE'
set -eu
test "$(hostname)" = imx93-tatsu
grep -qx 'IMG_VERSION="3.28.0.172"' /etc/os-release
test -s /home/root/xovi/exthome/qt-resource-rebuilder/hashtab
test -x /home/root/xovi/start
if test -S /tmp/qtfb.sock; then echo 'AppLoad is already running.'; exit 0; fi
systemd-run --quiet --no-block --collect --unit "ankimarkable-launcher-$(date +%s)" /home/root/xovi/start
for i in $(seq 1 60); do
  if test -S /tmp/qtfb.sock; then echo 'AppLoad is running.'; exit 0; fi
  sleep 1
done
/home/root/xovi/stock
echo 'Launcher did not become ready; restored stock interface.' >&2
exit 1
REMOTE
