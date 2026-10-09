#!/usr/bin/env bash
set -eo pipefail
source /opt/codex/environment-setup-*
set -u
read -r -a compiler <<< "$CXX"
read -r -a qt_flags <<< "$(pkg-config --cflags Qt6Gui Qt6Core)"
read -r -a qt_libs <<< "$(pkg-config --libs Qt6Gui Qt6Core)"
mkdir -p /out/objects
mapfile -t sources < <(find /microtex/src -name '*.cpp' | sort | grep -vE '/platform/(skia|gdi_win|cairo)/|/samples/')
sources+=(/tinyxml/tinyxml2.cpp /source/mathpng.cpp)
objects=()
for source_file in "${sources[@]}"; do
  object="/out/objects/$(printf '%s' "$source_file" | tr '/.' '__').o"
  "${compiler[@]}" -std=c++20 -O2 -DNDEBUG -DBUILD_QT -fPIC -I/microtex/src -I/tinyxml \
    "${qt_flags[@]}" -c "$source_file" -o "$object"
  objects+=("$object")
done
"${compiler[@]}" "${objects[@]}" "${qt_libs[@]}" -o /out/mathpng
$STRIP --strip-unneeded /out/mathpng
cp -R /microtex/res /out/microtex-res
cp /microtex/LICENSE /out/MicroTeX-LICENSE
cp /tinyxml/LICENSE.txt /out/tinyxml2-LICENSE.txt
file /out/mathpng
