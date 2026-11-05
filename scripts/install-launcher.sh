#!/usr/bin/env bash
set -euo pipefail

host=${1:-root@10.11.99.1}
ssh_options=(-o ConnectTimeout=10)
if [[ -n ${SSH_CONTROL_PATH:-} ]]; then ssh_options+=(-S "$SSH_CONTROL_PATH"); fi
identity=$(ssh "${ssh_options[@]}" "$host" 'printf "%s|%s|" "$(hostname)" "$(uname -m)"; sed -n '\''s/^IMG_VERSION="\(.*\)"/\1/p'\'' /etc/os-release')
[[ "$identity" == 'imx93-tatsu|aarch64|3.28.0.172' || "$identity" == 'imx93-tatsu|aarch64|3.29.1.0' ]] || {
  printf 'Unsupported tablet/firmware: %s. Tested: Paper Pure 3.28.0.172 or 3.29.1.0.\n' "$identity" >&2; exit 1;
}
ssh "${ssh_options[@]}" "$host" 'test ! -e /home/root/xovi' || {
  printf 'XOVI is already installed. Keep your existing launcher; this installer will not overwrite it.\n' >&2; exit 1;
}

stage=$(mktemp -d)
trap 'rm -rf "$stage"' EXIT
curl --fail --location --silent --show-error \
  https://github.com/asivery/rm-xovi-extensions/releases/download/v19-23052026/xovi-aarch64.tar.gz -o "$stage/xovi.tar.gz"
curl --fail --location --silent --show-error \
  https://github.com/asivery/rm-appload/releases/download/v0.6.0/appload-aarch64.tar.gz -o "$stage/appload.tar.gz"
verify() {
  local actual
  if command -v sha256sum >/dev/null; then actual=$(sha256sum "$1"); else actual=$(shasum -a 256 "$1"); fi
  [[ ${actual%% *} == "$2" ]] || { printf 'Checksum failed: %s\n' "$1" >&2; exit 1; }
}
verify "$stage/xovi.tar.gz" 32d64d1262ddc984e3235c7d0340a398fe6d5b3efa6a979865f5977b32630d27
verify "$stage/appload.tar.gz" 2e7912d15df910631dab847ea5d344081ad75c02a46aaac78bdf731accc53a1f
mkdir "$stage/runtime" "$stage/appload"
tar -xzf "$stage/xovi.tar.gz" -C "$stage/runtime"
tar -xzf "$stage/appload.tar.gz" -C "$stage/appload"
cp "$stage/appload/appload.so" "$stage/runtime/xovi/extensions.d/"
chmod +x "$stage/runtime/xovi/start" "$stage/runtime/xovi/stock"
tar -C "$stage/runtime" -cf - xovi | ssh "${ssh_options[@]}" "$host" 'tar -C /home/root -xf -; chown -R root:root /home/root/xovi'

script_dir=$(cd "$(dirname "$0")" && pwd)
ssh "${ssh_options[@]}" "$host" sh < "$script_dir/tablet-launcher-setup.sh"
