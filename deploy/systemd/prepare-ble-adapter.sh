#!/bin/sh
set -eu

DEVICE_PREFIX="${YUNDRONE_DEVICE_PREFIX:-yundrone}"
MAIN_CONF="/etc/bluetooth/main.conf"

ensure_general_key() {
    key="$1"
    value="$2"

    mkdir -p /etc/bluetooth
    touch "$MAIN_CONF"
    if ! grep -q '^\[General\]' "$MAIN_CONF"; then
        printf '\n[General]\n' >>"$MAIN_CONF"
    fi

    if grep -q "^${key}[[:space:]]*=" "$MAIN_CONF"; then
        sed -i "s/^${key}[[:space:]]*=.*/${key} = ${value}/" "$MAIN_CONF"
    else
        sed -i "/^\[General\]/a ${key} = ${value}" "$MAIN_CONF"
    fi
}

hostnamectl set-hostname --pretty "$DEVICE_PREFIX" || true
ensure_general_key "Name" "$DEVICE_PREFIX"
ensure_general_key "ControllerMode" "dual"

if command -v btmgmt >/dev/null 2>&1; then
    # Keep the controller in the vendor default dual-mode shape. On some
    # combo Wi-Fi/Bluetooth controllers, forcing LE-only/static-addr made
    # iOS/macOS connect but then stall before ATT MTU exchange completed.
    btmgmt power off || true
    btmgmt bredr on || true
    btmgmt connectable on || true
    btmgmt bondable off || true
    btmgmt power on || true
fi
