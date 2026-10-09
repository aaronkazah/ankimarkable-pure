#!/usr/bin/env bash
set -euo pipefail
root=$(cd "$(dirname "$0")/.." && pwd)
docker build --platform linux/arm64 -t ankimarkable-pure-builder:1.95.0 "$root"
docker run --rm --platform linux/arm64 -e CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-3}" \
  -v "$root:/project" -v ankimarkable-pure-cargo:/usr/local/cargo -v ankimarkable-pure-build:/build \
  ankimarkable-pure-builder:1.95.0 cargo test --locked --all-targets
