#!/usr/bin/env bash
set -euo pipefail

# Sync current Rust repository to remote Orin machine using rsync.
# - Uses .gitignore rules automatically when .gitignore exists.
# - Always excludes .git metadata and local target/ compilation folders.

TARGET_DEFAULT="orin-Mocap5G:/opt/ble-command-gateway/"
# TARGET_DEFAULT="orangepi:~/work/ble-command-gateway/"
TARGET="${1:-$TARGET_DEFAULT}"
REMOTE_SUDO="${RSYNC_REMOTE_SUDO:-0}"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

RSYNC_OPTS=(
  -azP
  --delete
  --human-readable
  --exclude=.git/
  --filter='- target/'
  --filter='- Cargo.lock'
)

if [[ "$REMOTE_SUDO" == "1" ]]; then
  RSYNC_OPTS+=(--rsync-path='sudo rsync')
fi

if [[ -f .gitignore ]]; then
  RSYNC_OPTS+=(--filter=':- .gitignore')
else
  echo "[WARN] .gitignore not found. Only .git/ will be excluded."
fi

echo "[INFO] Source : ${SCRIPT_DIR}/"
echo "[INFO] Target : ${TARGET}"
if [[ "$REMOTE_SUDO" == "1" ]]; then
  echo "[INFO] Remote rsync path: sudo rsync"
fi

if [[ "$TARGET" =~ ^([^:]+):(.+)$ ]]; then
  REMOTE_HOST="${BASH_REMATCH[1]}"
  REMOTE_PATH="${BASH_REMATCH[2]}"

  if [[ "$REMOTE_SUDO" != "1" ]]; then
    if ! ssh -o ConnectTimeout=8 "$REMOTE_HOST" "mkdir -p \"$REMOTE_PATH\" >/dev/null 2>&1"; then
      echo "[ERROR] Remote path is not writable: ${REMOTE_HOST}:${REMOTE_PATH}"
      echo "[HINT ] Use a writable path (e.g. ~/work/ble-command-gateway/) or run:"
      echo "        RSYNC_REMOTE_SUDO=1 $0 \"$TARGET\""
      exit 1
    fi
  fi
fi

echo "[INFO] Running: rsync ${RSYNC_OPTS[*]} \"${SCRIPT_DIR}/\" \"${TARGET}\""
rsync "${RSYNC_OPTS[@]}" "${SCRIPT_DIR}/" "$TARGET"

echo "[OK] Sync completed."
