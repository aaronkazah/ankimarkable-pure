#!/usr/bin/env bash
set -euo pipefail
root=$(cd "$(dirname "$0")/.." && pwd)
host=${1:-root@10.11.99.1}
bundle=${2:-$root/dist/ankimarkable-pure}
ssh_options=(-o ConnectTimeout=10)
if [[ -n ${SSH_CONTROL_PATH:-} ]]; then ssh_options+=(-S "$SSH_CONTROL_PATH"); fi
for file in ankimarkable-pure ankimarkable-pure-tools mathpng external.manifest.json icon.png LICENSE; do
  [[ -s "$bundle/$file" ]] || { printf 'Missing bundle file: %s\n' "$file" >&2; exit 1; }
done
identity=$(ssh "${ssh_options[@]}" "$host" 'printf "%s|%s|" "$(hostname)" "$(uname -m)"; sed -n '\''s/^IMG_VERSION="\(.*\)"/\1/p'\'' /etc/os-release')
[[ "$identity" == 'imx93-tatsu|aarch64|3.28.0.172' ]] || { printf 'Unsupported tablet: %s\n' "$identity" >&2; exit 1; }
ssh "${ssh_options[@]}" "$host" sh <<'REMOTE'
set -eu
test -s /home/root/xovi/extensions.d/appload.so || { echo 'Install AppLoad first.' >&2; exit 1; }
app=/home/root/xovi/exthome/appload/ankimarkable-pure
for executable in /proc/[0-9]*/exe; do
  if test "$(readlink "$executable" 2>/dev/null || true)" = "$app/ankimarkable-pure"; then
    echo 'Close Anki before updating its application files.' >&2; exit 1
  fi
done
mkdir -p "$app" /home/root/.ankimarkable-pure
chmod 700 /home/root/.ankimarkable-pure
REMOTE
tar -C "$bundle" -cf - . | ssh "${ssh_options[@]}" "$host" 'tar -C /home/root/xovi/exthome/appload/ankimarkable-pure -xf -'
ssh "${ssh_options[@]}" "$host" sh <<'REMOTE'
set -eu
app=/home/root/xovi/exthome/appload/ankimarkable-pure
chmod 755 "$app/ankimarkable-pure" "$app/ankimarkable-pure-tools" "$app/mathpng"
chown -R root:root "$app"
collection=/home/root/.ankimarkable-pure/collection.anki2
if test ! -f "$collection"; then "$app/ankimarkable-pure-tools" create-demo "$collection"; fi
"$app/ankimarkable-pure-tools" check "$collection"
echo 'Installed. Open AppLoad, then Anki — Paper Pure.'
REMOTE
