#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
VERSION="${VERSION:-$(tr -d '\r\n' <"$ROOT_DIR/VERSION")}"
PLATFORM="${PLATFORM:-}"
OUT_DIR="${OUT_DIR:-$ROOT_DIR/dist/ble-client/releases/$VERSION}"

detect_platform() {
  case "$(uname -s):$(uname -m)" in
    Darwin:arm64|Darwin:aarch64) printf '%s' "macos-arm64" ;;
    Linux:x86_64|Linux:amd64) printf '%s' "linux-amd64" ;;
    Linux:aarch64|Linux:arm64) printf '%s' "linux-arm64" ;;
    *) printf '%s' "unsupported" ;;
  esac
}

if [ -z "$PLATFORM" ]; then
  PLATFORM="$(detect_platform)"
fi
[ "$PLATFORM" != "unsupported" ] || {
  echo "error: unsupported platform $(uname -s) $(uname -m)" >&2
  exit 1
}

cd "$ROOT_DIR"
cargo build --release -p yundrone-ble-client

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
mkdir -p "$tmp/package" "$OUT_DIR"
cp "$ROOT_DIR/target/release/yundrone-ble-client" "$tmp/package/yundrone-ble-client"
chmod +x "$tmp/package/yundrone-ble-client"
printf '%s\n' "$VERSION" >"$tmp/package/VERSION"

tarball="$OUT_DIR/yundrone-ble-client-${PLATFORM}.tar.gz"
if tar --version 2>/dev/null | grep -qi 'gnu tar'; then
  tar --format=ustar --no-xattrs -czf "$tarball" -C "$tmp/package" .
else
  COPYFILE_DISABLE=1 tar -czf "$tarball" -C "$tmp/package" .
fi
sha="$(shasum -a 256 "$tarball" | awk '{print $1}')"

cat >"$OUT_DIR/yundrone-ble-client-${PLATFORM}.json" <<EOF
{
  "platform": "${PLATFORM}",
  "version": "${VERSION}",
  "file": "yundrone-ble-client-${PLATFORM}.tar.gz",
  "sha256": "${sha}"
}
EOF

echo "$tarball"
