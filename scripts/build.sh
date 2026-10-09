#!/usr/bin/env bash
set -euo pipefail
root=$(cd "$(dirname "$0")/.." && pwd)
stage=$(mktemp -d)
trap 'rm -rf "$stage"' EXIT
verify() {
  local actual
  if command -v sha256sum >/dev/null; then actual=$(sha256sum "$1"); else actual=$(shasum -a 256 "$1"); fi
  [[ ${actual%% *} == "$2" ]] || { printf 'Checksum failed: %s\n' "$1" >&2; exit 1; }
}
docker build --platform linux/arm64 -t ankimarkable-pure-builder:1.95.0 "$root"
docker run --rm --platform linux/arm64 -e CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-3}" \
  -v "$root:/project" -v ankimarkable-pure-cargo:/usr/local/cargo -v ankimarkable-pure-build:/build \
  ankimarkable-pure-builder:1.95.0 cargo build --locked --release --bins

math_image=ankimarkable-pure-math-builder:3.28.0.172
if ! docker image inspect "$math_image" >/dev/null 2>&1; then
  mkdir "$stage/sdk"
  curl --fail --location --silent --show-error \
    https://storage.googleapis.com/remarkable-codex-toolchain/3.28.0.172/remarkable-production-image-5.8.203-tatsu-public-aarch64-toolchain.sh -o "$stage/sdk/SDK.sh"
  verify "$stage/sdk/SDK.sh" 90186bbbea7219002ad93b2197c753ac87284bebdde14f8eaeac27dbd9e11fcb
  docker build --platform linux/arm64 --build-context "sdk=$stage/sdk" \
    -f "$root/tools/mathpng/Dockerfile" -t "$math_image" "$root/tools/mathpng"
fi
docker run --rm --platform linux/arm64 -v "$root:/project:ro" "$math_image" python3 /project/tests/installer_contract.py
curl --fail --location --silent --show-error https://codeload.github.com/NanoMichael/MicroTeX/tar.gz/0e3707f6dafebb121d98b53c64364d16fefe481d -o "$stage/microtex.tar.gz"
curl --fail --location --silent --show-error https://codeload.github.com/leethomason/tinyxml2/tar.gz/refs/tags/10.0.0 -o "$stage/tinyxml.tar.gz"
verify "$stage/microtex.tar.gz" 47476269d29c41df322bce6bdd2daa7017cc50b9eda841b2bec1767dba28daa6
verify "$stage/tinyxml.tar.gz" 3bdf15128ba16686e69bce256cc468e76c7b94ff2c7f391cc5ec09e40bff3839
tar -xzf "$stage/microtex.tar.gz" -C "$stage"
tar -xzf "$stage/tinyxml.tar.gz" -C "$stage"
mkdir "$stage/math"
docker run --rm --platform linux/arm64 --user "$(id -u):$(id -g)" -v "$root/tools/mathpng:/source:ro" \
  -v "$stage/MicroTeX-0e3707f6dafebb121d98b53c64364d16fefe481d:/microtex:ro" \
  -v "$stage/tinyxml2-10.0.0:/tinyxml:ro" -v "$stage/math:/out" "$math_image" bash /source/build.sh

bundle="$root/dist/ankimarkable-pure"
mkdir -p "$bundle/licenses"
docker run --rm --platform linux/arm64 --user "$(id -u):$(id -g)" -v ankimarkable-pure-build:/build:ro -v "$bundle:/out" \
  ankimarkable-pure-builder:1.95.0 sh -c 'cp /build/target/release/ankimarkable-pure /build/target/release/ankimarkable-pure-tools /out/'
cp "$stage/math/mathpng" "$bundle/"
rm -rf "$bundle/microtex-res"
cp -R "$stage/math/microtex-res" "$bundle/"
cp -R "$root/assets/licenses/." "$bundle/licenses/"
cp "$stage/math/MicroTeX-LICENSE" "$stage/math/tinyxml2-LICENSE.txt" "$bundle/licenses/"
cp "$root/packaging/external.manifest.json" "$root/assets/icon.png" "$root/LICENSE" "$root/README.md" "$bundle/"
cp -R "$root/scripts" "$bundle/"
tar -C "$root/dist" -czf "$root/dist/ankimarkable-pure-0.2.0-paper-pure.tar.gz" ankimarkable-pure
(cd "$root/dist" && if command -v sha256sum >/dev/null; then sha256sum ankimarkable-pure-0.2.0-paper-pure.tar.gz; else shasum -a 256 ankimarkable-pure-0.2.0-paper-pure.tar.gz; fi) > "$root/dist/SHA256SUMS"
printf '\nInstallable package: %s\n' "$root/dist/ankimarkable-pure-0.2.0-paper-pure.tar.gz"
