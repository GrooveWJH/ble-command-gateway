#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 1 ]]; then
  echo "Usage: $0 <capture-dir-or-tar.gz>" >&2
  exit 1
fi

input="$1"
workdir=""

cleanup() {
  if [[ -n "$workdir" && -d "$workdir" && "$workdir" == /tmp/* ]]; then
    rm -rf "$workdir"
  fi
}

trap cleanup EXIT

if [[ -d "$input" ]]; then
  workdir="$input"
elif [[ -f "$input" ]]; then
  workdir="$(mktemp -d /tmp/ble-int-extract-XXXXXX)"
  tar -xzf "$input" -C "$workdir"
else
  echo "Input not found: $input" >&2
  exit 1
fi

server_log="$workdir/server-journal.log"
dbus_log="$workdir/dbus-monitor.log"
btmon_log="$workdir/btmon.log"
bluetoothd_log="$workdir/bluetoothd-journal.log"

echo "== Server Timeline =="
grep -nE 'ble.server.starting|ble.advertising|ble.gatt.ready' "$server_log" || true

echo
echo "== D-Bus Timeline =="
grep -nEA1 -B1 \
  'RegisterAdvertisement|GetAll|org\.bluez\.LEAdvertisement1|LocalName|MinInterval|MaxInterval' \
  "$dbus_log" || true

echo
echo "== MGMT/HCI Timeline =="
grep -nE 'Add Extended Advertising Parameters|LE Set Extended Advertising Parameters|LE Set Advertising Parameters|Min advertising interval|Max advertising interval|Properties:|Duration:|Timeout:|Instance:' \
  "$btmon_log" || true

echo
echo "== bluetoothd Journal =="
grep -nE 'advertis|Advertising|bluetoothd' "$bluetoothd_log" || true
