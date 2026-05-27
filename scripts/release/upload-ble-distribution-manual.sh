#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
REMOTE="${YUNDRONE_INSTALL_REMOTE:-self-cloudserver}"
REMOTE_ROOT="${YUNDRONE_INSTALL_REMOTE_ROOT:-/var/www/install.yundrone.cn}"
VERSION="${VERSION:-$(tr -d '\r\n' <"$ROOT_DIR/VERSION")}"
DIST_DIR="${DIST_DIR:-$ROOT_DIR/dist/install-site}"
SUDO_PASSWORD="${YUNDRONE_INSTALL_REMOTE_SUDO_PASSWORD:-}"

remote_sudo() {
  if [ -n "$SUDO_PASSWORD" ]; then
    ssh "$REMOTE" "printf '%s\n' '$SUDO_PASSWORD' | sudo -S $*"
  else
    ssh "$REMOTE" "sudo $*"
  fi
}

stage_launcher() {
  local launcher_dist="$DIST_DIR/yundrone/ble/launcher"
  rm -rf "$launcher_dist"
  mkdir -p "$launcher_dist/installer/versions/$VERSION"
  cp -R "$ROOT_DIR/scripts/install/launcher/." "$launcher_dist/installer/versions/$VERSION/"
  python3 "$ROOT_DIR/scripts/release/generate_installer_manifest.py" \
    --source "$ROOT_DIR/scripts/install/launcher" \
    --version "$VERSION" \
    --base-url "https://install.yundrone.cn/yundrone/ble/launcher" \
    --output "$launcher_dist/installer/latest.json"
  cp "$launcher_dist/installer/latest.json" "$launcher_dist/installer/versions/$VERSION/MANIFEST.json"
  cp "$ROOT_DIR/scripts/install/ble-wifi-tool.sh" "$DIST_DIR/ble-wifi-tool.sh"
  cp "$ROOT_DIR/scripts/install/ble-server.sh" "$DIST_DIR/ble-server.sh"
}

stage_server_installer() {
  local server_dist="$DIST_DIR/yundrone/ble-server"
  mkdir -p "$server_dist/installer/versions/$VERSION"
  cp -R "$ROOT_DIR/scripts/install/installer/." "$server_dist/installer/versions/$VERSION/"
  python3 "$ROOT_DIR/scripts/release/generate_installer_manifest.py" \
    --source "$ROOT_DIR/scripts/install/installer" \
    --version "$VERSION" \
    --base-url "https://install.yundrone.cn/yundrone/ble-server" \
    --output "$server_dist/installer/latest.json"
  cp "$server_dist/installer/latest.json" "$server_dist/installer/versions/$VERSION/MANIFEST.json"
  cp "$ROOT_DIR/scripts/install/ble-server.sh" "$server_dist/install.sh"
}

stage_gum_tools() {
  local gum_source="${GUM_TOOLS_DIR:-$ROOT_DIR/dist/gum-tools}"
  if [ ! -f "$gum_source/latest.json" ]; then
    python3 "$ROOT_DIR/scripts/release/package-gum-tools.py" \
      --output-dir "$gum_source" \
      --base-url "https://install.yundrone.cn/yundrone/ble/tools/gum"
  fi
  mkdir -p "$DIST_DIR/yundrone/ble/tools"
  cp -R "$gum_source" "$DIST_DIR/yundrone/ble/tools/gum"
}

stage_client_release() {
  local client_source="${CLIENT_RELEASE_DIR:-$ROOT_DIR/dist/ble-client/releases/$VERSION}"
  local client_dist="$DIST_DIR/yundrone/ble-client/releases"
  if compgen -G "$client_source/yundrone-ble-client-*.tar.gz" >/dev/null; then
    mkdir -p "$client_dist/$VERSION"
    cp "$client_source"/yundrone-ble-client-*.tar.gz "$client_dist/$VERSION/"
    python3 "$ROOT_DIR/scripts/release/generate_client_release_metadata.py" \
      --version "$VERSION" \
      --release-dir "$client_source" \
      --base-url "https://install.yundrone.cn/yundrone/ble-client" \
      --output "$client_dist/latest.json"
    cp "$client_dist/latest.json" "$client_dist/$VERSION/release.json"
  else
    echo "warn: no client tarballs found in $client_source; skipping client release metadata" >&2
  fi
}

stage_server_release() {
  local server_source="${SERVER_RELEASE_DIR:-$ROOT_DIR/dist/ble-server/releases/$VERSION}"
  local server_dist="$DIST_DIR/yundrone/ble-server"
  if compgen -G "$server_source/yundrone-ble-server-*.tar.gz" >/dev/null; then
    mkdir -p "$server_dist/releases/$VERSION"
    cp "$server_source"/yundrone-ble-server-*.tar.gz "$server_dist/releases/$VERSION/"
    python3 "$ROOT_DIR/scripts/release/generate_server_release_metadata.py" \
      --version "$VERSION" \
      --release-dir "$server_source" \
      --base-url "https://install.yundrone.cn/yundrone/ble-server" \
      --output "$server_dist/latest.json"
    cp "$server_dist/latest.json" "$server_dist/releases/$VERSION/release.json"
  else
    echo "warn: no server tarballs found in $server_source; skipping server release metadata" >&2
  fi
}

upload_dist() {
  if tar --version 2>/dev/null | grep -qi 'gnu tar'; then
    tar --format=ustar --no-xattrs -C "$DIST_DIR" -czf "$DIST_DIR.tar.gz" .
  else
    COPYFILE_DISABLE=1 tar -C "$DIST_DIR" -czf "$DIST_DIR.tar.gz" .
  fi
  scp "$DIST_DIR.tar.gz" "$REMOTE:/tmp/yundrone-install-site.tar.gz"
  remote_sudo "mkdir -p '$REMOTE_ROOT'"
  remote_sudo "tar -xzf /tmp/yundrone-install-site.tar.gz -C '$REMOTE_ROOT'"
  remote_sudo "rm -f /tmp/yundrone-install-site.tar.gz"
}

main() {
  rm -rf "$DIST_DIR"
  mkdir -p "$DIST_DIR"
  stage_launcher
  stage_server_installer
  stage_gum_tools
  stage_client_release
  stage_server_release
  upload_dist
  curl -fsSL https://install.yundrone.cn/ble-wifi-tool.sh >/dev/null
  curl -fsSL https://install.yundrone.cn/ble-server.sh >/dev/null
  echo "uploaded installer entrypoints to https://install.yundrone.cn"
}

main "$@"
