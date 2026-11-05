#!/usr/bin/env bash
set -euo pipefail
host=${1:-root@10.11.99.1}
ssh_options=(-o ConnectTimeout=10)
if [[ -n ${SSH_CONTROL_PATH:-} ]]; then ssh_options+=(-S "$SSH_CONTROL_PATH"); fi
ssh "${ssh_options[@]}" "$host" sh <<'CHECK'
set -eu
test "$(hostname)" = imx93-tatsu
grep -Eq '^IMG_VERSION="(3\.28\.0\.172|3\.29\.1\.0)"$' /etc/os-release
CHECK
if ! ssh "${ssh_options[@]}" "$host" sh <<'CHECK'
set -eu
map=/home/root/xovi/exthome/qt-resource-rebuilder
version=$(sed -n 's/^IMG_VERSION="\(.*\)"/\1/p' /etc/os-release)
test -s "$map/hashtab"
test -f "$map/hashtab-firmware"
test "$(cat "$map/hashtab-firmware")" = "$version"
CHECK
then
  echo 'Rebuilding the launcher resource map for this firmware.'
  script_dir=$(cd "$(dirname "$0")" && pwd)
  ssh "${ssh_options[@]}" "$host" sh < "$script_dir/tablet-launcher-setup.sh"
fi
ssh "${ssh_options[@]}" "$host" sh <<'REMOTE'
set -eu
test "$(hostname)" = imx93-tatsu
grep -Eq '^IMG_VERSION="(3\.28\.0\.172|3\.29\.1\.0)"$' /etc/os-release
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
