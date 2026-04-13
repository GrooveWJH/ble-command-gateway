#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PROFILE="${PROFILE:-release}"
APP_NAME="YunDrone BLE Gateway.app"
APP_DIR="$ROOT_DIR/target/$PROFILE/$APP_NAME"
BINARY_PATH="$ROOT_DIR/target/$PROFILE/gui"
PLIST_PATH="$ROOT_DIR/crates/gui/macos/Info.plist"

if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "error: this packaging command only supports macOS" >&2
  exit 1
fi

if ! command -v cargo >/dev/null 2>&1; then
  echo "error: cargo not found in PATH" >&2
  exit 1
fi

if ! command -v codesign >/dev/null 2>&1; then
  echo "error: codesign not found in PATH" >&2
  exit 1
fi

echo "==> Building gui ($PROFILE)"
cargo build "--$PROFILE" -p gui

echo "==> Staging app bundle"
rm -rf "$APP_DIR"
mkdir -p "$APP_DIR/Contents/MacOS"
cp "$BINARY_PATH" "$APP_DIR/Contents/MacOS/gui"
cp "$PLIST_PATH" "$APP_DIR/Contents/Info.plist"

echo "==> Signing app bundle"
codesign --force --deep --sign - "$APP_DIR"

echo "==> Packaged app"
echo "$APP_DIR"
