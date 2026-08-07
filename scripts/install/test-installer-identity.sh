#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
TMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_DIR"' EXIT

PREFIX="yundrone"
ADAPTER=""
BLUETOOTH_CLASS_DIR="$TMP_DIR/bluetooth"
LEGACY_IDENTITY_FILE="$TMP_DIR/ble-device-name"
mkdir -p "$BLUETOOTH_CLASS_DIR"

have() { command -v "$1" >/dev/null 2>&1; }
run_root() { "$@"; }

# shellcheck source=/dev/null
. "$ROOT_DIR/scripts/install/installer/lib/state.sh"
# shellcheck source=/dev/null
. "$ROOT_DIR/scripts/install/installer/commands/core.sh"

assert_eq() {
  [ "$1" = "$2" ] || {
    printf 'expected %s, got %s\n' "$2" "$1" >&2
    exit 1
  }
}

mkdir -p "$BLUETOOTH_CLASS_DIR/hci1" "$BLUETOOTH_CLASS_DIR/hci0"
printf '%s\n' 'AA:BB:CC:11:22:33' >"$BLUETOOTH_CLASS_DIR/hci1/address"
printf '%s\n' 'DC:A6:32:12:AB:CD' >"$BLUETOOTH_CLASS_DIR/hci0/address"
assert_eq "$(default_adapter_name)" "hci0"
assert_eq "$(identity_serial)" "12abcd"
assert_eq "$(identity_name)" "yundrone-12abcd"

printf '%s\n' '00:00:00:00:00:00' >"$BLUETOOTH_CLASS_DIR/hci0/address"
assert_eq "$(identity_name)" "yundrone-null"

rm -rf "$BLUETOOTH_CLASS_DIR/hci0"
assert_eq "$(default_adapter_name)" "hci1"
assert_eq "$(identity_name)" "yundrone-112233"

printf '%s\n' '00:00:00:00:00:00' >"$BLUETOOTH_CLASS_DIR/hci1/address"
assert_eq "$(identity_serial)" "null"
assert_eq "$(identity_name)" "yundrone-null"

printf '%s\n' 'legacy-name' >"$LEGACY_IDENTITY_FILE"
cleanup_legacy_identity_file
[ ! -e "$LEGACY_IDENTITY_FILE" ] || exit 1

if bash "$ROOT_DIR/scripts/install/installer/main.sh" --name-alias lab1 >"$TMP_DIR/out" 2>&1; then
  printf '%s\n' 'removed --name-alias option unexpectedly succeeded' >&2
  exit 1
fi
grep -q '未知参数: --name-alias' "$TMP_DIR/out"

printf '%s\n' 'installer identity tests passed'
