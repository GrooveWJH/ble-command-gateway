have() {
  command -v "$1" >/dev/null 2>&1
}

need_sudo() {
  if [ "$(id -u)" -eq 0 ]; then
    return 0
  fi
  have sudo || fail "需要 sudo 权限。请使用 root 或具备 sudo 权限的用户执行。"
  sudo -v || fail "sudo 验证失败"
}

ensure_sudo_step() {
  if [ "$(id -u)" -eq 0 ]; then
    if tui_ready; then
      gum style --foreground 42 "✓ 已获得系统权限：root"
    else
      ok "已获得系统权限：root"
    fi
    return 0
  fi

  have sudo || fail "需要 sudo 权限。请使用 root 或具备 sudo 权限的用户执行。"

  if tui_ready; then
    tui_info_card "需要系统权限

接下来会写入安装目录、更新 systemd service，并启动 YunDrone BLE Server。
请在下面的 sudo 提示中输入当前用户密码。"
    gum style --foreground 39 "• 获取系统权限"
  else
    info "需要 sudo 权限，接下来会请求当前用户密码。"
  fi

  sudo -v || fail "sudo 验证失败"

  if tui_ready; then
    gum style --foreground 42 "✓ 系统权限验证通过"
  else
    ok "系统权限验证通过"
  fi
}

run_root() {
  if [ "$(id -u)" -eq 0 ]; then
    "$@"
  else
    sudo "$@"
  fi
}

with_timeout() {
  local seconds="$1"
  shift
  if have timeout; then
    timeout "${seconds}s" "$@"
  else
    "$@"
  fi
}

run_root_with_timeout() {
  local seconds="$1"
  shift
  if [ "$(id -u)" -eq 0 ]; then
    with_timeout "$seconds" "$@"
  elif have timeout; then
    sudo timeout "${seconds}s" "$@"
  else
    sudo "$@"
  fi
}

run_root_no_block_systemctl() {
  if [ "$(id -u)" -eq 0 ]; then
    systemctl --no-block "$@"
  else
    sudo systemctl --no-block "$@"
  fi
}

safe_arch() {
  local machine
  if [ "$(uname -s)" != "Linux" ]; then
    printf '%s' "unsupported"
    return 0
  fi
  machine="$(uname -m)"
  case "$machine" in
    aarch64|arm64)
      printf '%s' "linux-arm64"
      ;;
    x86_64|amd64)
      printf '%s' "linux-amd64"
      ;;
    *)
      printf '%s' "unsupported"
      ;;
  esac
}

detect_arch() {
  local arch
  arch="$(safe_arch)"
  [ "$arch" != "unsupported" ] || fail "当前系统不支持安装：$(uname -s) $(uname -m)。仅支持 Linux arm64/amd64。"
  printf '%s' "$arch"
}

fetch() {
  local url="$1"
  local output="$2"
  local mode="${3:-quiet}"
  debug "fetch url=${url}"
  debug "fetch output=${output}"
  if have curl; then
    if [ "$mode" = "progress" ] && [ -t 1 ]; then
      curl -fL --progress-bar "$url" -o "$output"
    else
      curl -fsSL "$url" -o "$output"
    fi
  elif have wget; then
    if [ "$mode" = "progress" ] && [ -t 1 ]; then
      wget --show-progress -O "$output" "$url"
    else
      wget -qO "$output" "$url"
    fi
  else
    fail "需要 curl 或 wget"
  fi
}

sha256_file() {
  local file="$1"
  if have sha256sum; then
    sha256sum "$file" | awk '{print $1}'
  elif have shasum; then
    shasum -a 256 "$file" | awk '{print $1}'
  else
    fail "需要 sha256sum 或 shasum"
  fi
}

json_value() {
  local key="$1"
  local file="$2"
  python3 - "$key" "$file" <<'PY'
import json
import sys

key = sys.argv[1]
path = sys.argv[2]
with open(path, "r", encoding="utf-8") as fh:
    data = json.load(fh)

value = data
for part in key.split("."):
    value = value[part]
print(value)
PY
}

tool_ok() {
  have "$1" && printf '%s' "OK" || printf '%s' "缺失"
}

ensure_python() {
  have python3 || fail "需要 python3 解析 release metadata"
}

ensure_basic_tools() {
  local missing=""
  for tool in tar systemctl; do
    if ! have "$tool"; then
      missing="${missing} ${tool}"
    fi
  done
  if ! have sha256sum && ! have shasum; then
    missing="${missing} sha256sum/shasum"
  fi
  [ -z "$missing" ] || fail "缺少必要工具:${missing}"
}

maybe_install_dependencies() {
  local missing=""
  for tool in curl tar systemctl bluetoothctl python3; do
    if ! have "$tool"; then
      missing="${missing} ${tool}"
    fi
  done
  if ! have sha256sum && ! have shasum; then
    missing="${missing} sha256sum/shasum"
  fi
  if [ -z "$missing" ]; then
    return 0
  fi

  warn "缺少工具:${missing}"
  if have apt-get; then
    confirm_yes "是否使用 apt-get 安装 bluez network-manager ca-certificates curl tar python3？" || fail "依赖不完整，已取消安装"
    run_root apt-get update
    run_root apt-get install -y bluez network-manager ca-certificates curl tar python3
  else
    fail "当前自动安装依赖只支持 Debian / Ubuntu / Armbian"
  fi
}
