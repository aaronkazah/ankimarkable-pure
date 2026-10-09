#!/usr/bin/env bash
set -euo pipefail
root=$(cd "$(dirname "$0")/.." && pwd)
case "$(uname -s)-$(uname -m)" in
  Darwin-arm64) platform=macos-arm64; digest=a1d0c7985759accca0bf12e51ac1dcbf0f6cf2fffb62e6e0f62d091c477a10a3 ;;
  Linux-aarch64) platform=linux-arm64; digest=69b1378b8133192d7d2feb12a116fa12d035594f58db3eff215879e4ad8cf39b ;;
  *) echo 'Regenerate styles on an ARM64 Mac or Linux machine.' >&2; exit 1 ;;
esac
compiler=$(mktemp)
trap 'rm -f "$compiler"' EXIT
curl --fail --location --silent --show-error \
  "https://github.com/tailwindlabs/tailwindcss/releases/download/v3.4.17/tailwindcss-$platform" -o "$compiler"
if command -v sha256sum >/dev/null; then actual=$(sha256sum "$compiler"); else actual=$(shasum -a 256 "$compiler"); fi
[[ ${actual%% *} == "$digest" ]] || { echo 'Tailwind compiler checksum failed.' >&2; exit 1; }
chmod 700 "$compiler"
cd "$root"
"$compiler" -c tailwind.config.cjs -i assets/ui.input.css -o assets/ui.css --minify
