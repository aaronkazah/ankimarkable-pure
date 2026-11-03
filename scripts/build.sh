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
docker run --rm --platform linux/arm64 -e CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-2}" \
  -v "$root:/project" -v ankimarkable-pure-cargo:/usr/local/cargo -v ankimarkable-pure-build:/build \
  ankimarkable-pure-builder:1.95.0 cargo build --locked --release --bins

math_image=ankimarkable-pure-math-builder:3.28.0.172
"$root/scripts/prepare-sdk.sh"
docker run --rm --platform linux/arm64 -v "$root:/project:ro" \
  -v ankimarkable-pure-build:/build:ro -e ANKIMARKABLE_TEST_BINARY=/build/target/release/ankimarkable-pure \
  "$math_image" python3 /project/tests/frontend_contract.py
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

bundle="$stage/package/ankimarkable-pure"
mkdir -p "$bundle/licenses" "$bundle/backend"
docker run --rm --platform linux/arm64 --user "$(id -u):$(id -g)" -v ankimarkable-pure-build:/build:ro -v "$bundle:/out" \
  ankimarkable-pure-builder:1.95.0 sh -c 'cp /build/target/release/ankimarkable-pure /build/target/release/ankimarkable-pure-tools /out/'
docker run --rm --platform linux/arm64 --user "$(id -u):$(id -g)" -v "$root/frontend:/frontend:ro" -v "$bundle:/out" "$math_image" \
  /opt/codex/sysroots/aarch64-codexsdk-linux/usr/libexec/rcc --binary /frontend/application.qrc -o /out/resources.rcc
cp "$stage/math/mathpng" "$bundle/"
cp -R "$stage/math/microtex-res" "$bundle/"
cp -R "$root/assets/licenses/." "$bundle/licenses/"
cp "$stage/math/MicroTeX-LICENSE" "$stage/math/tinyxml2-LICENSE.txt" "$bundle/licenses/"
cp "$root/packaging/manifest.json" "$root/packaging/launcher-control.qmd" "$root/assets/icon.png" "$root/LICENSE" "$root/README.md" "$root/ankipure" "$bundle/"
cp "$root/packaging/backend-entry.sh" "$bundle/backend/entry"
chmod 755 "$bundle/backend/entry"
cp -R "$root/scripts" "$bundle/"
cp -R "$root/docs" "$bundle/"
version=$(sed -n 's/^version = "\(.*\)"/\1/p' "$root/Cargo.toml" | head -n 1)
printf '%s\n' "$version" > "$bundle/VERSION"
mkdir -p "$root/dist"
COPYFILE_DISABLE=1 tar -C "$stage/package" -czf "$root/dist/ankimarkable-pure-$version-paper-pure.tar.gz" ankimarkable-pure
(cd "$root/dist" && if command -v sha256sum >/dev/null; then sha256sum "ankimarkable-pure-$version-paper-pure.tar.gz"; else shasum -a 256 "ankimarkable-pure-$version-paper-pure.tar.gz"; fi) > "$root/dist/SHA256SUMS"
if test -d "$root/dist/ankimarkable-pure"; then mv "$root/dist/ankimarkable-pure" "$stage/previous-bundle"; fi
mv "$bundle" "$root/dist/ankimarkable-pure"
printf '\nInstallable package: %s\n' "$root/dist/ankimarkable-pure-$version-paper-pure.tar.gz"
