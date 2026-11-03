#!/usr/bin/env bash
set -euo pipefail
root=$(cd "$(dirname "$0")/.." && pwd)
docker build --platform linux/arm64 -t ankimarkable-pure-builder:1.95.0 "$root"
docker run --rm --platform linux/arm64 -e CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-2}" \
  -v "$root:/project" -v ankimarkable-pure-cargo:/usr/local/cargo -v ankimarkable-pure-build:/build \
  ankimarkable-pure-builder:1.95.0 cargo test --locked --all-targets
docker run --rm --platform linux/arm64 -e CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-2}" \
  -v "$root:/project" -v ankimarkable-pure-cargo:/usr/local/cargo -v ankimarkable-pure-build:/build \
  ankimarkable-pure-builder:1.95.0 cargo build --locked --bin ankimarkable-pure
docker run --rm --platform linux/arm64 -v "$root:/project:ro" \
  ankimarkable-pure-builder:1.95.0 python3 /project/tests/cli_contract.py
docker run --rm --platform linux/arm64 -v "$root:/project:ro" \
  ankimarkable-pure-builder:1.95.0 python3 /project/tests/desktop_contract.py
docker run --rm --platform linux/arm64 -v "$root:/project:ro" \
  ankimarkable-pure-builder:1.95.0 python3 /project/tests/installer_contract.py
"$root/scripts/prepare-sdk.sh"
docker run --rm --platform linux/arm64 -v "$root:/project:ro" \
  -v ankimarkable-pure-build:/build:ro -e ANKIMARKABLE_TEST_BINARY=/build/target/debug/ankimarkable-pure \
  ankimarkable-pure-math-builder:3.28.0.172 python3 /project/tests/frontend_contract.py

docker run --rm --platform linux/arm64 -v "$root:/project:ro" \
  ankimarkable-pure-math-builder:3.28.0.172 python3 /project/tests/startup_contract.py
