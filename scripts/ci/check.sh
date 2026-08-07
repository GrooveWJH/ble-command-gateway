#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
MODE="${1:-quality}"

cd "$ROOT_DIR"

run_quality() {
  bash -n scripts/install/ble-wifi-tool.sh
  bash -n scripts/install/ble-server.sh
  bash -n scripts/install/launcher/main.sh scripts/install/launcher/lib/*.sh
  bash -n scripts/install/installer/main.sh scripts/install/installer/lib/*.sh scripts/install/installer/commands/*.sh
  bash scripts/install/test-installer-identity.sh
  python3 -m py_compile scripts/release/*.py
  python3 scripts/release/check_version_consistency.py
  python3 -m unittest discover -s scripts/release -p 'test_*.py'
  cargo fmt --all --check
  cargo test --locked --workspace --exclude yundrone-ble-server
  cargo test --locked -p yundrone-ble-server
  cargo clippy --locked --workspace --all-targets --exclude yundrone-ble-server -- -D warnings
  cargo clippy --locked -p yundrone-ble-server --all-targets -- -D warnings
}

run_build_full() {
  cargo build --locked --workspace
}

run_build_desktop() {
  cargo build --locked -p protocol -p platform_runtime -p yundrone-ble-client -p gui
}

run_package_macos() {
  chmod +x scripts/package-macos-gui.sh
  ./scripts/package-macos-gui.sh
  /usr/bin/codesign --verify --deep --strict --verbose=2 \
    "target/release/yundrone-ble-client.app"
}

case "$MODE" in
  quality)
    run_quality
    ;;
  build-full)
    run_build_full
    ;;
  build-desktop)
    run_build_desktop
    ;;
  package-macos)
    run_package_macos
    ;;
  *)
    cat >&2 <<'EOF'
usage: scripts/ci/check.sh <mode>

Modes:
  quality        Run the same fmt/test/clippy/version checks as CI Quality.
  build-full     Build the full workspace, matching Linux CI build jobs.
  build-desktop  Build desktop packages, matching macOS/Windows CI build jobs.
  package-macos  Build and verify the macOS app bundle, matching release CI.
EOF
    exit 2
    ;;
esac
