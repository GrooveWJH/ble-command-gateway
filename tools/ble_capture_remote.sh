#!/usr/bin/env bash
set -euo pipefail

if [[ $# -lt 2 || $# -gt 4 ]]; then
  echo "Usage: $0 <ssh-host> <sudo-password> [wait-seconds] [service-name]" >&2
  exit 1
fi

host="$1"
sudo_password="$2"
wait_secs="${3:-130}"
service_name="${4:-ble-command-gateway.service}"
stamp="$(date +%Y%m%d-%H%M%S)"
local_root="${TMPDIR:-/tmp}/ble-int-captures/${host}-${stamp}"

mkdir -p "$local_root"

remote_tar="$(ssh "$host" bash -s -- "$sudo_password" "$wait_secs" "$service_name" <<'SH'
set -euo pipefail

pass="$1"
wait_secs="$2"
service_name="$3"
outdir="$(mktemp -d /tmp/ble-int-debug-XXXXXX)"

cleanup() {
  for pid in "${server_pid:-}" "${bluetooth_pid:-}" "${dbus_pid:-}" "${btmon_pid:-}"; do
    if [[ -n "${pid:-}" ]] && kill -0 "$pid" 2>/dev/null; then
      kill "$pid" 2>/dev/null || true
      wait "$pid" 2>/dev/null || true
    fi
  done
}

trap cleanup EXIT

journalctl -u "$service_name" -f -o short-iso >"$outdir/server-journal.log" 2>&1 &
server_pid=$!

journalctl -u bluetooth.service -f -o short-iso >"$outdir/bluetoothd-journal.log" 2>&1 &
bluetooth_pid=$!

echo "$pass" | sudo -S -p '' timeout "$((wait_secs + 15))" dbus-monitor --system \
  >"$outdir/dbus-monitor.log" 2>&1 &
dbus_pid=$!

echo "$pass" | sudo -S -p '' timeout "$((wait_secs + 15))" btmon -T \
  >"$outdir/btmon.log" 2>&1 &
btmon_pid=$!

sleep 2
echo "$pass" | sudo -S -p '' systemctl restart "$service_name"
sleep "$wait_secs"

cleanup
trap - EXIT

tar -C "$outdir" -czf "$outdir.tar.gz" .
echo "$outdir.tar.gz"
SH
)"

scp "$host:$remote_tar" "$local_root/capture.tar.gz" >/dev/null
tar -xzf "$local_root/capture.tar.gz" -C "$local_root"

echo "capture_dir=$local_root"
echo "capture_tar=$local_root/capture.tar.gz"
