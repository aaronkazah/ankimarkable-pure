#!/usr/bin/env bash
set -euo pipefail
host=${1:?Provide an SSH host}
action=${2:?Choose enable or disable}
[[ $action == enable || $action == disable ]] || { echo 'Choose enable or disable.' >&2; exit 2; }
ssh_options=(-o ConnectTimeout=10)
if [[ -n ${SSH_CONTROL_PATH:-} ]]; then ssh_options+=(-S "$SSH_CONTROL_PATH"); fi
ssh "${ssh_options[@]}" "$host" sh -s -- "$action" <<'REMOTE'
set -eu
action=$1
name=ankimarkable-start.service
unit=/etc/systemd/system/$name
launcher=/home/root/xovi/exthome/appload/ankimarkable-pure/scripts/boot-launcher.sh
test "$(hostname)" = imx93-tatsu || { echo 'This tablet is not a Paper Pure.' >&2; exit 1; }
stage=$(mktemp -d)
read_only=0
finish() {
    sync
    if test "$read_only" = 1; then mount -o remount,ro / 2>/dev/null || true; fi
    rm -rf "$stage"
}
trap finish EXIT
if test "$action" = enable; then
    grep -qx 'IMG_VERSION="3.28.0.172"' /etc/os-release || { echo 'Automatic startup supports firmware 3.28.0.172 only.' >&2; exit 1; }
    test -x "$launcher" || { echo 'Install Anki first with ./ankipure setup.' >&2; exit 1; }
    cat > "$stage/$name" <<UNIT
[Unit]
Description=Start Anki's app launcher after unlocking Paper Pure
After=home.mount xochitl.service
Requires=home.mount
Wants=xochitl.service
ConditionPathExists=$launcher

[Service]
Type=oneshot
ExecStart=$launcher
RemainAfterExit=yes
TimeoutStartSec=90

[Install]
WantedBy=multi-user.target
UNIT
    SYSTEMD_UNIT_PATH=/run/systemd/generator:/run/systemd/generator.early:/run/systemd/generator.late: \
        systemd-analyze verify "$stage/$name"
fi
mounted() { awk -v target="$1" '$2 == target {found=1} END {exit !found}' /proc/mounts; }
if awk '$2 == "/" && $4 ~ /(^|,)ro(,|$)/ {found=1} END {exit !found}' /proc/mounts; then
    read_only=1
    mount -o remount,rw /
fi
# /etc is an overlay that keeps changes in memory until the next restart.
# Unmount it so the change reaches the root file system and persists.
if mounted /etc; then umount -R /etc 2>/dev/null || true; fi
if mounted /etc; then echo 'Could not make the change permanent. Restart the tablet, then retry.' >&2; exit 1; fi
if test "$action" = enable; then
    cp "$stage/$name" "$unit"
    chmod 644 "$unit"
    systemctl daemon-reload
    systemctl enable "$name"
    echo 'Automatic startup is on. The tablet restarts in a few seconds; unlock it and Anki appears in the sidebar.'
else
    systemctl disable "$name" 2>/dev/null || true
    rm -f "$unit"
    systemctl daemon-reload
    echo 'Automatic startup is off. The tablet restarts in a few seconds with its normal interface.'
fi
# The restart restores the /etc overlay. The delay lets this connection close first.
systemd-run --quiet --on-active=3 /bin/sh -c 'systemctl reboot'
REMOTE
