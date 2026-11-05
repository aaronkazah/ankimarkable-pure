#!/usr/bin/env bash
set -euo pipefail
root=$(cd "$(dirname "$0")/.." && pwd)
host=${1:-root@10.11.99.1}
bundle=${2:-$root/dist/ankimarkable-pure}
ssh_options=(-o ConnectTimeout=10 -o ServerAliveInterval=5 -o ServerAliveCountMax=3)
if [[ -n ${SSH_CONTROL_PATH:-} ]]; then ssh_options+=(-S "$SSH_CONTROL_PATH"); fi
for file in ankimarkable-pure ankimarkable-pure-tools mathpng manifest.json resources.rcc launcher-control.qmd backend/entry icon.png LICENSE; do
  [[ -s "$bundle/$file" ]] || { printf 'Missing bundle file: %s\n' "$file" >&2; exit 1; }
done
identity=$(ssh "${ssh_options[@]}" "$host" 'printf "%s|%s|" "$(hostname)" "$(uname -m)"; sed -n '\''s/^IMG_VERSION="\(.*\)"/\1/p'\'' /etc/os-release')
[[ "$identity" == 'imx93-tatsu|aarch64|3.28.0.172' || "$identity" == 'imx93-tatsu|aarch64|3.29.1.0' ]] || { printf 'Unsupported tablet: %s\n' "$identity" >&2; exit 1; }
ssh "${ssh_options[@]}" "$host" sh <<'REMOTE'
set -eu
test -s /home/root/xovi/extensions.d/appload.so || { echo 'Install AppLoad first.' >&2; exit 1; }
app=/home/root/xovi/exthome/appload/ankimarkable-pure
for executable in /proc/[0-9]*/exe; do
  if test "$(readlink "$executable" 2>/dev/null || true)" = "$app/ankimarkable-pure"; then
    echo 'Close Anki before updating its application files.' >&2; exit 1
  fi
done
mkdir -p /home/root/.ankimarkable-pure /home/root/.cache
chmod 700 /home/root/.ankimarkable-pure
REMOTE
stage=$(ssh "${ssh_options[@]}" "$host" 'umask 077; mktemp -d /home/root/.cache/ankimarkable-pure.XXXXXX')
[[ "$stage" =~ ^/home/root/\.cache/ankimarkable-pure\.[[:alnum:]]{6}$ ]] || { echo 'Invalid staging directory.' >&2; exit 1; }
tar -C "$bundle" -cf - . | ssh "${ssh_options[@]}" "$host" "tar -C '$stage' -xf -"
ssh "${ssh_options[@]}" "$host" sh -s -- "$stage" <<'REMOTE'
set -eu
stage=$1
app=/home/root/xovi/exthome/appload/ankimarkable-pure
for file in ankimarkable-pure ankimarkable-pure-tools mathpng manifest.json resources.rcc launcher-control.qmd backend/entry icon.png LICENSE; do
  test -s "$stage/$file" || { echo "Incomplete transfer: $file" >&2; exit 1; }
done
if test -f "$app/external.manifest.json"; then
  "$stage/ankimarkable-pure-tools" migrate-config "$app/external.manifest.json" /home/root/.ankimarkable-pure
fi
chmod 755 "$stage/ankimarkable-pure" "$stage/ankimarkable-pure-tools" "$stage/mathpng" "$stage/backend/entry"
chown -R root:root "$stage"
collection=/home/root/.ankimarkable-pure/collection.anki2
if test ! -f "$collection"; then "$stage/ankimarkable-pure-tools" create-demo "$collection"; fi
"$stage/ankimarkable-pure-tools" check "$collection" >/dev/null
mkdir -p /home/root/xovi/exthome/appload
previous="$stage.previous"
if test -d "$app"; then mv "$app" "$previous"; fi
if ! mv "$stage" "$app"; then
  test ! -d "$previous" || mv "$previous" "$app"
  echo 'Could not activate the new application.' >&2; exit 1
fi
test ! -d "$previous" || rm -rf "$previous"
hook=/home/root/xovi/exthome/qt-resource-rebuilder/ankimarkable-pure-control.qmd
first_hook=0
cmp -s "$app/launcher-control.qmd" "$hook" || first_hook=1
cp "$app/launcher-control.qmd" "$hook"
test -f /home/root/.ankimarkable-pure/launcher-command.json || printf '{}\n' > /home/root/.ankimarkable-pure/launcher-command.json
if test "$first_hook" = 1; then systemctl restart xochitl; fi
echo 'Installed. The CLI will open Anki.'
REMOTE
