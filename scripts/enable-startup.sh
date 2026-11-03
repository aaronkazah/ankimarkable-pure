#!/usr/bin/env bash
set -euo pipefail
host=${1:-root@10.11.99.1}
ssh_options=(-o ConnectTimeout=10)
if [[ -n ${SSH_CONTROL_PATH:-} ]]; then ssh_options+=(-S "$SSH_CONTROL_PATH"); fi
ssh "${ssh_options[@]}" "$host" sh <<'REMOTE'
set -eu
test "$(hostname)" = imx93-tatsu
grep -qx 'IMG_VERSION="3.28.0.172"' /etc/os-release
app=/home/root/xovi/exthome/appload/ankimarkable-pure
test -x "$app/scripts/boot-launcher.sh"
unit=ankimarkable-start.service
stage=$(mktemp -d)
trap 'rm -rf "$stage"' EXIT
cat > "$stage/$unit" <<'UNIT'
[Unit]
Description=Start Anki's app launcher after unlocking Paper Pure
After=home.mount xochitl.service
Requires=home.mount
Wants=xochitl.service
ConditionPathExists=/home/root/xovi/exthome/appload/ankimarkable-pure/scripts/boot-launcher.sh

[Service]
Type=oneshot
ExecStart=/home/root/xovi/exthome/appload/ankimarkable-pure/scripts/boot-launcher.sh
RemainAfterExit=yes
TimeoutStartSec=90

[Install]
WantedBy=multi-user.target
UNIT
SYSTEMD_UNIT_PATH=/run/systemd/generator:/run/systemd/generator.early:/run/systemd/generator.late: \
    systemd-analyze verify "$stage/$unit"
was_readonly=0
if awk '$2 == "/" && $4 ~ /(^|,)ro(,|$)/ {found=1} END {exit !found}' /proc/mounts; then
    was_readonly=1
    mount -o remount,rw /
fi
cleanup() {
    if test "$was_readonly" = 1; then mount -o remount,ro /; fi
    rm -rf "$stage"
}
trap cleanup EXIT INT TERM
cp "$stage/$unit" "/etc/systemd/system/$unit"
chmod 644 "/etc/systemd/system/$unit"
systemctl daemon-reload
systemctl enable "$unit"
systemctl start "$unit"
echo 'Anki launcher startup enabled for firmware 3.28.0.172.'
REMOTE
