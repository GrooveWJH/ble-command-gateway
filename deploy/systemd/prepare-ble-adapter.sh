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

run_btmgmt() {
    action="$1"
    shift
    if command -v timeout >/dev/null 2>&1; then
        if timeout 5s btmgmt "$@"; then
            printf 'ble.adapter.prepare.btmgmt.%s ok\n' "$action"
        else
            status="$?"
            printf 'ble.adapter.prepare.btmgmt.%s skipped status=%s\n' "$action" "$status" >&2
        fi
    elif btmgmt "$@"; then
        printf 'ble.adapter.prepare.btmgmt.%s ok\n' "$action"
    else
        status="$?"
        printf 'ble.adapter.prepare.btmgmt.%s skipped status=%s\n' "$action" "$status" >&2
    fi
}

if command -v btmgmt >/dev/null 2>&1; then
    # Keep the controller in the vendor default dual-mode shape. On some
    # combo Wi-Fi/Bluetooth controllers, forcing LE-only/static-addr made
    # iOS/macOS connect but then stall before ATT MTU exchange completed.
    run_btmgmt power_off power off
    run_btmgmt bredr_on bredr on
    run_btmgmt connectable_on connectable on
    run_btmgmt bondable_off bondable off
    run_btmgmt power_on power on
fi
