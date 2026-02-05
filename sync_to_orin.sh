#!/usr/bin/env bash
set -euo pipefail

# Sync current repository to remote Orin machine using rsync.
# - Uses .gitignore rules automatically when .gitignore exists.
# - Always excludes .git metadata.
# - Protects remote .venv/ from deletion.

TARGET_DEFAULT="orin-Mocap5G:~/work/ble-wifi-provisioning/"
TARGET="${1:-$TARGET_DEFAULT}"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

RSYNC_OPTS=(
  -azP
  --delete
  --human-readable
  --exclude=.git/
  --filter='P .venv/'
)

if [[ -f .gitignore ]]; then
  RSYNC_OPTS+=(--filter=':- .gitignore')
else
  echo "[WARN] .gitignore not found. Only .git/ will be excluded."
fi

echo "[INFO] Source : ${SCRIPT_DIR}/"
echo "[INFO] Target : ${TARGET}"
echo "[INFO] Protect: remote .venv/ will not be deleted"

echo "[INFO] Running: rsync ${RSYNC_OPTS[*]} \"${SCRIPT_DIR}/\" \"${TARGET}\""
rsync "${RSYNC_OPTS[@]}" "${SCRIPT_DIR}/" "$TARGET"

echo "[OK] Sync completed."
