#!/usr/bin/env bash
set -Eeuo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

LAUNCHER_BASE_URL="${YUNDRONE_BLE_LAUNCHER_BASE_URL:-https://install.yundrone.cn/yundrone/ble/launcher}"
SERVER_ENTRY_URL="${YUNDRONE_BLE_SERVER_ENTRY_URL:-https://install.yundrone.cn/ble-server.sh}"
CLIENT_BASE_URL="${YUNDRONE_BLE_CLIENT_BASE_URL:-https://install.yundrone.cn/yundrone/ble-client}"
TOOLS_BASE_URL="${YUNDRONE_BLE_TOOLS_BASE_URL:-https://install.yundrone.cn/yundrone/ble/tools}"
CLIENT_CACHE_ROOT="${YUNDRONE_BLE_CLIENT_CACHE_ROOT:-${HOME}/.yundrone/ble-client}"
GUM_CACHE_ROOT="${YUNDRONE_BLE_GUM_CACHE_ROOT:-${HOME}/.yundrone/ble-launcher/tools/gum}"
COMMAND="menu"
ASSUME_YES="no"
VERBOSE="${YUNDRONE_VERBOSE:-no}"

# shellcheck source=/dev/null
. "${SCRIPT_DIR}/lib/ui.sh"
# shellcheck source=/dev/null
. "${SCRIPT_DIR}/lib/platform.sh"
# shellcheck source=/dev/null
. "${SCRIPT_DIR}/lib/download.sh"
# shellcheck source=/dev/null
. "${SCRIPT_DIR}/lib/gum.sh"
# shellcheck source=/dev/null
. "${SCRIPT_DIR}/lib/client.sh"
# shellcheck source=/dev/null
. "${SCRIPT_DIR}/lib/server.sh"

usage() {
  cat <<EOF
YunDrone BLE 统一入口

用法:
  ble.sh [命令] [选项]

命令:
  menu      打开统一 TUI，默认动作
  client    下载/启动 YunDrone BLE Client CLI
  server    打开 BLE Server 部署/管理向导
  doctor    检查 launcher、client、server 安装源

选项:
  --yes               跳过确认，适合自动化
  --verbose, -v       输出下载、缓存、校验和路径细节
  -h, --help          显示帮助

示例:
  bash <(curl -fsSL https://install.yundrone.cn/ble.sh)
  bash <(curl -fsSL https://install.yundrone.cn/ble.sh) -- client
  bash <(curl -fsSL https://install.yundrone.cn/ble.sh) -- server
  bash <(curl -fsSL https://install.yundrone.cn/ble.sh) -- doctor
EOF
}

parse_args() {
  if [ "${1:-}" = "--" ]; then
    shift
  fi

  if [ $# -gt 0 ]; then
    case "$1" in
      menu|client|server|doctor)
        COMMAND="$1"
        shift
        ;;
    esac
  fi

  while [ $# -gt 0 ]; do
    case "$1" in
      --yes|-y)
        ASSUME_YES="yes"
        shift
        ;;
      --verbose|-v)
        VERBOSE="yes"
        shift
        ;;
      -h|--help)
        usage
        exit 0
        ;;
      *)
        fail "未知参数: $1"
        ;;
    esac
  done
}

render_home() {
  tui_clear
  tui_title
  local platform client_state server_state
  platform="$(detect_platform)"
  client_state="$(client_status_label)"
  server_state="$(server_status_label)"
  tui_card "当前设备概览

平台：${platform}
Client：${client_state}
Server：${server_state}
Client 缓存：${CLIENT_CACHE_ROOT}

你可以在电脑上启动 Client，也可以在 Linux 边缘设备上部署 Server。" 39
}

menu() {
  ensure_gum
  local choice
  while true; do
    render_home
    choice="$(printf '%s\n' \
      "启动 BLE Client CLI" \
      "部署 / 管理本机 BLE Server" \
      "运行环境诊断" \
      "退出" | tui_choose "选择要做的事")" || exit 0
    case "$choice" in
      "启动 BLE Client CLI") client_menu ;;
      "部署 / 管理本机 BLE Server") server_menu ;;
      运行环境诊断) doctor ;;
      退出) exit 0 ;;
    esac
  done
}

doctor() {
  if use_tui; then
    ensure_gum
    tui_clear
    tui_title
    {
      printf '检查项,结果\n'
      printf '平台,%s\n' "$(detect_platform)"
      printf 'Bash,%s\n' "$BASH_VERSION"
      printf 'curl/wget,%s\n' "$(if have curl || have wget; then echo OK; else echo 缺失; fi)"
      printf 'python3,%s\n' "$(tool_ok python3)"
      printf 'sha256sum/shasum,%s\n' "$(if have_sha256; then echo OK; else echo 缺失; fi)"
      printf 'tar,%s\n' "$(tool_ok tar)"
      printf 'gum,%s\n' "$(if [ -x "${GUM_BIN:-}" ]; then echo 已缓存; else echo 可下载; fi)"
      printf 'client 安装源,%s\n' "$(if url_ok "${CLIENT_BASE_URL}/releases/latest.json"; then echo OK; else echo 不可访问; fi)"
      printf 'server 入口,%s\n' "$(if url_ok "$SERVER_ENTRY_URL"; then echo OK; else echo 不可访问; fi)"
      printf 'server 本机支持,%s\n' "$(if is_linux; then echo 支持; else echo 仅 Linux 支持; fi)"
    } | gum table --border rounded --separator "," --columns "检查项,结果"
    tui_pause
    return 0
  fi

  section "YunDrone BLE 入口诊断"
  field_line "平台" "$(detect_platform)"
  field_line "Bash" "$BASH_VERSION"
  field_line "curl/wget" "$(if have curl || have wget; then echo OK; else echo 缺失; fi)"
  field_line "python3" "$(tool_ok python3)"
  field_line "sha256sum/shasum" "$(if have_sha256; then echo OK; else echo 缺失; fi)"
  field_line "tar" "$(tool_ok tar)"
  field_line "client 安装源" "$(if url_ok "${CLIENT_BASE_URL}/releases/latest.json"; then echo OK; else echo 不可访问; fi)"
  field_line "server 入口" "$(if url_ok "$SERVER_ENTRY_URL"; then echo OK; else echo 不可访问; fi)"
}

main() {
  parse_args "$@"
  case "$COMMAND" in
    menu) menu ;;
    client) client_launch ;;
    server) server_menu ;;
    doctor) doctor ;;
    *) fail "未知命令: $COMMAND" ;;
  esac
}

main "$@"
