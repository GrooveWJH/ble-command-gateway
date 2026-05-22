#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
VERSION="${VERSION:-$(tr -d '\r\n' <"$ROOT_DIR/VERSION") }"
VERSION="${VERSION//[[:space:]]/}"
PLATFORMS="${PLATFORMS:-linux-amd64}"
IMAGE="${YUNDRONE_RUST_LINUX_IMAGE:-rust:1.95-bookworm}"
CARGO_CACHE="${YUNDRONE_DOCKER_CARGO_CACHE:-$HOME/.cache/yundrone-docker-cargo}"
TARGET_CACHE="${YUNDRONE_DOCKER_TARGET_CACHE:-$ROOT_DIR/target-docker}"

mkdir -p "$CARGO_CACHE" "$TARGET_CACHE" "$ROOT_DIR/dist/ble-client/releases/$VERSION"

for platform in $PLATFORMS; do
  case "$platform" in
    linux-amd64) docker_platform="linux/amd64" ;;
    linux-arm64) docker_platform="linux/arm64" ;;
    *) echo "error: unsupported platform: $platform" >&2; exit 2 ;;
  esac

  echo "==> Building yundrone-ble-client for $platform with Docker platform $docker_platform"
  docker run --rm --platform "$docker_platform" \
    -e PLATFORM="$platform" \
    -e VERSION="$VERSION" \
    -e CARGO_TERM_COLOR=always \
    -e PATH="/usr/local/cargo/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin" \
    -v "$ROOT_DIR:/work" \
    -v "$CARGO_CACHE:/usr/local/cargo/registry" \
    -v "$TARGET_CACHE/$platform:/work/target" \
    -w /work \
    "$IMAGE" \
    bash -c '
      set -euo pipefail
      apt-get update
      apt-get install -y --no-install-recommends pkg-config libdbus-1-dev libudev-dev ca-certificates tar
      cargo build --release -p yundrone-ble-client
      tmp="$(mktemp -d)"
      mkdir -p "$tmp/package" "/work/dist/ble-client/releases/$VERSION"
      cp /work/target/release/yundrone-ble-client "$tmp/package/yundrone-ble-client"
      chmod +x "$tmp/package/yundrone-ble-client"
      printf "%s\n" "$VERSION" > "$tmp/package/VERSION"
      tar --format=ustar --no-xattrs -czf "/work/dist/ble-client/releases/$VERSION/yundrone-ble-client-$PLATFORM.tar.gz" -C "$tmp/package" .
      rm -rf "$tmp"
    '
done
