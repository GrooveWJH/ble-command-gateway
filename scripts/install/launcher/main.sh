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
UI_LANG="${YUNDRONE_BLE_LANG:-}"

# shellcheck source=/dev/null
. "${SCRIPT_DIR}/lib/ui.sh"
# shellcheck source=/dev/null
. "${SCRIPT_DIR}/lib/platform.sh"
# shellcheck source=/dev/null
. "${SCRIPT_DIR}/lib/download.sh"
# shellcheck source=/dev/null
. "${SCRIPT_DIR}/lib/i18n.sh"
# shellcheck source=/dev/null
. "${SCRIPT_DIR}/lib/gum.sh"
# shellcheck source=/dev/null
. "${SCRIPT_DIR}/lib/client.sh"
# shellcheck source=/dev/null
. "${SCRIPT_DIR}/lib/server.sh"

usage() {
  if [ "${UI_LANG:-zh}" = "en" ]; then
    cat <<EOF_USAGE
YunDrone BLE Wi-Fi Tool

Usage:
  ble-wifi-tool.sh [command] [options]

Commands:
  menu      Open the unified TUI, default action
  client    Install/update and launch the BLE client CLI
  server    Open the controlled-device installer
  doctor    Check launcher, client, and controlled-device install sources

Options:
  --lang <zh|en>      Interface language
  --yes               Skip confirmations for automation
  --verbose, -v       Print download, cache, checksum, and path details
  -h, --help          Show help

Examples:
  bash <(curl -fsSL https://install.yundrone.cn/ble-wifi-tool.sh)
  bash <(curl -fsSL https://install.yundrone.cn/ble-wifi-tool.sh) -- --lang en
  bash <(curl -fsSL https://install.yundrone.cn/ble-wifi-tool.sh) -- client
  bash <(curl -fsSL https://install.yundrone.cn/ble-wifi-tool.sh) -- server
EOF_USAGE
  else
    cat <<EOF_USAGE
YunDrone BLE Wi-Fi 工具

用法:
  ble-wifi-tool.sh [命令] [选项]

命令:
  menu      打开统一 TUI，默认动作
  client    安装/升级并启动 BLE 客户端 CLI
  server    打开被控端部署/管理向导
  doctor    检查启动器、客户端、被控端安装源

选项:
  --lang <zh|en>      界面语言
  --yes               跳过确认，适合自动化
  --verbose, -v       输出下载、缓存、校验和路径细节
  -h, --help          显示帮助

示例:
  bash <(curl -fsSL https://install.yundrone.cn/ble-wifi-tool.sh)
  bash <(curl -fsSL https://install.yundrone.cn/ble-wifi-tool.sh) -- --lang en
  bash <(curl -fsSL https://install.yundrone.cn/ble-wifi-tool.sh) -- client
  bash <(curl -fsSL https://install.yundrone.cn/ble-wifi-tool.sh) -- server
EOF_USAGE
  fi
}

parse_args() {
  if [ "${1:-}" = "--" ]; then
    shift
  fi

  while [ $# -gt 0 ]; do
    case "$1" in
      menu|client|server|doctor)
        COMMAND="$1"
        shift
        ;;
      --lang)
        UI_LANG="${2:-}"
        [ "$UI_LANG" = "zh" ] || [ "$UI_LANG" = "en" ] || fail "--lang must be zh or en"
        shift 2
        ;;
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
        fail "未知参数 / Unknown argument: $1"
        ;;
    esac
  done
}

choose_language() {
  if [ -n "${UI_LANG:-}" ]; then
    return 0
  fi
  if ! use_tui; then
    UI_LANG="zh"
    return 0
  fi

  ensure_gum
  tui_clear
  gum style \
    --foreground 15 \
    --background 24 \
    --bold \
    --width 76 \
    --padding "0 2" \
    --margin "1 0 0 2" \
    "YunDrone BLE Wi-Fi Tool"
  local choice
  choice="$(printf '%s\n' "中文" "English" | tui_choose_raw "请选择界面语言 / Choose language")" || exit 0
  case "$choice" in
    English) UI_LANG="en" ;;
    *) UI_LANG="zh" ;;
  esac
}

render_home() {
  tui_clear
  tui_title
  local platform client_state controlled_state
  platform="$(detect_platform)"
  client_state="$(client_status_label)"
  controlled_state="$(controlled_status_label)"
  tui_card "$(printf '%s\n\n%s：%s\n%s：%s\n%s：%s\n%s：%s\n\n%s' \
    "$(tr_text "当前设备概览" "Current device overview")" \
    "$(tr_text "平台" "Platform")" "$platform" \
    "$(tr_text "客户端" "Client")" "$client_state" \
    "$(tr_text "被控端" "Controlled device")" "$controlled_state" \
    "$(tr_text "客户端缓存" "Client cache")" "$CLIENT_CACHE_ROOT" \
    "$(tr_text "你可以在电脑上启动客户端，也可以在 Linux 边缘设备上部署被控端。" "Launch the client on this computer, or deploy the controlled-device service on a Linux edge device.")")" 39
}

menu() {
  ensure_gum
  choose_language
  local choice
  while true; do
    render_home
    choice="$(printf '%s\n' \
      "$(tr_text "启动交互式客户端" "Launch interactive client")" \
      "$(tr_text "部署 / 管理本机被控端" "Deploy / manage local controlled device")" \
      "$(tr_text "运行环境诊断" "Run diagnostics")" \
      "$(tr_text "退出" "Exit")" | tui_choose "$(tr_text "选择要做的事" "Choose an action")")" || exit 0
    case "$choice" in
      "$(tr_text "启动交互式客户端" "Launch interactive client")") client_launch ;;
      "$(tr_text "部署 / 管理本机被控端" "Deploy / manage local controlled device")") controlled_menu ;;
      "$(tr_text "运行环境诊断" "Run diagnostics")") doctor ;;
      "$(tr_text "退出" "Exit")") exit 0 ;;
    esac
  done
}

doctor() {
  choose_language
  if use_tui; then
    ensure_gum
    tui_clear
    tui_title
    {
      printf '%s,%s\n' "$(tr_text "检查项" "Check")" "$(tr_text "结果" "Result")"
      printf '%s,%s\n' "$(tr_text "平台" "Platform")" "$(detect_platform)"
      printf '%s,%s\n' "Bash" "$BASH_VERSION"
      printf '%s,%s\n' "curl/wget" "$(if have curl || have wget; then tr_text OK OK; else tr_text 缺失 Missing; fi)"
      printf '%s,%s\n' "python3" "$(tool_ok python3)"
      printf '%s,%s\n' "sha256sum/shasum" "$(if have_sha256; then tr_text OK OK; else tr_text 缺失 Missing; fi)"
      printf '%s,%s\n' "tar" "$(tool_ok tar)"
      printf '%s,%s\n' "gum" "$(if [ -x "${GUM_BIN:-}" ]; then tr_text 已缓存 Cached; else tr_text 可下载 Downloadable; fi)"
      printf '%s,%s\n' "$(tr_text "客户端安装源" "Client source")" "$(if url_ok "${CLIENT_BASE_URL}/releases/latest.json"; then tr_text OK OK; else tr_text 不可访问 Unreachable; fi)"
      printf '%s,%s\n' "$(tr_text "被控端入口" "Controlled-device entry")" "$(if url_ok "$SERVER_ENTRY_URL"; then tr_text OK OK; else tr_text 不可访问 Unreachable; fi)"
      printf '%s,%s\n' "$(tr_text "被控端本机支持" "Controlled-device local support")" "$(if is_linux; then tr_text 支持 Supported; else tr_text "仅 Linux 支持" "Linux only"; fi)"
    } | gum table --border rounded --separator "," --columns "$(tr_text "检查项,结果" "Check,Result")"
    tui_pause
    return 0
  fi

  section "$(tr_text "YunDrone BLE Wi-Fi 工具诊断" "YunDrone BLE Wi-Fi Tool Diagnostics")"
  field_line "$(tr_text "平台" "Platform")" "$(detect_platform)"
  field_line "Bash" "$BASH_VERSION"
  field_line "curl/wget" "$(if have curl || have wget; then tr_text OK OK; else tr_text 缺失 Missing; fi)"
  field_line "python3" "$(tool_ok python3)"
  field_line "sha256sum/shasum" "$(if have_sha256; then tr_text OK OK; else tr_text 缺失 Missing; fi)"
  field_line "tar" "$(tool_ok tar)"
  field_line "$(tr_text "客户端安装源" "Client source")" "$(if url_ok "${CLIENT_BASE_URL}/releases/latest.json"; then tr_text OK OK; else tr_text 不可访问 Unreachable; fi)"
  field_line "$(tr_text "被控端入口" "Controlled-device entry")" "$(if url_ok "$SERVER_ENTRY_URL"; then tr_text OK OK; else tr_text 不可访问 Unreachable; fi)"
}

main() {
  parse_args "$@"
  case "$COMMAND" in
    menu) menu ;;
    client) choose_language; client_launch ;;
    server) choose_language; controlled_menu ;;
    doctor) doctor ;;
    *) fail "未知命令 / Unknown command: $COMMAND" ;;
  esac
}

main "$@"
