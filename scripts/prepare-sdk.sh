#!/usr/bin/env bash
set -euo pipefail
root=$(cd "$(dirname "$0")/.." && pwd)
image=ankimarkable-pure-math-builder:3.28.0.172
if docker image inspect "$image" >/dev/null 2>&1; then exit 0; fi
stage=$(mktemp -d)
trap 'rm -rf "$stage"' EXIT
mkdir "$stage/sdk"
curl --fail --location --silent --show-error \
  https://storage.googleapis.com/remarkable-codex-toolchain/3.28.0.172/remarkable-production-image-5.8.203-tatsu-public-aarch64-toolchain.sh -o "$stage/sdk/SDK.sh"
if command -v sha256sum >/dev/null; then checksum=$(sha256sum "$stage/sdk/SDK.sh");
else checksum=$(shasum -a 256 "$stage/sdk/SDK.sh"); fi
[[ ${checksum%% *} == 90186bbbea7219002ad93b2197c753ac87284bebdde14f8eaeac27dbd9e11fcb ]] || { echo 'SDK checksum failed.' >&2; exit 1; }
docker build --platform linux/arm64 --build-context "sdk=$stage/sdk" \
  -f "$root/tools/mathpng/Dockerfile" -t "$image" "$root/tools/mathpng"
