#!/usr/bin/env bash
set -Eeuo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

INSTALL_BASE_URL="${YUNDRONE_INSTALL_BASE_URL:-https://install.yundrone.cn/yundrone/ble-server}"
INSTALL_ROOT="/opt/yundrone/ble-command-gateway"
SERVICE_NAME="yundrone-ble-command-gateway.service"
SERVICE_PATH="/etc/systemd/system/${SERVICE_NAME}"
STATE_DIR="/var/lib/yundrone"
IDENTITY_FILE="${STATE_DIR}/ble-device-name"
DEFAULT_PREFIX="yundrone"
DEFAULT_BACKEND="bluez-dbus"
LOG_LINES=120
SERVICE_START_TIMEOUT=60

COMMAND="menu"
PREFIX="$DEFAULT_PREFIX"
VERSION="latest"
ADAPTER=""
BACKEND="$DEFAULT_BACKEND"
ASSUME_YES="no"
PURGE="no"
RESET_NAME="no"
VERBOSE="${YUNDRONE_VERBOSE:-no}"

# shellcheck source=/dev/null
. "${SCRIPT_DIR}/lib/ui.sh"
# shellcheck source=/dev/null
. "${SCRIPT_DIR}/lib/platform.sh"
# shellcheck source=/dev/null
. "${SCRIPT_DIR}/lib/state.sh"
# shellcheck source=/dev/null
. "${SCRIPT_DIR}/lib/release.sh"
# shellcheck source=/dev/null
. "${SCRIPT_DIR}/lib/systemd.sh"
# shellcheck source=/dev/null
. "${SCRIPT_DIR}/lib/gum.sh"
# shellcheck source=/dev/null
. "${SCRIPT_DIR}/lib/tui.sh"
# shellcheck source=/dev/null
. "${SCRIPT_DIR}/commands/core.sh"
# shellcheck source=/dev/null
. "${SCRIPT_DIR}/lib/menu.sh"

usage() {
  cat <<EOF
YunDrone BLE Server 安装器

用法:
  ble-server.sh [命令] [选项]

命令:
  menu         打开智能安装向导，默认动作
  install      安装或更新 YunDrone BLE Server
  update       等同 install
  uninstall    卸载服务和程序文件，默认保留 BLE 名称
  reinstall    卸载后重新安装，默认保留 BLE 名称
  status       显示安装状态、服务状态和 BLE 名称
  doctor       只做环境诊断，不安装
  logs         显示最近服务日志
  reset-name   删除持久化 BLE 名称并重启服务

选项:
  --prefix <name>       BLE 名前缀，默认: yundrone
  --version <version>   指定安装版本，默认: latest
  --adapter <hciX>      蓝牙适配器提示，默认自动检测
  --backend <name>      广播后端，默认: bluez-dbus
  --yes                 跳过确认，用于自动化
  --purge               卸载时同时删除 /var/lib/yundrone
  --reset-name          安装/重装时重新生成 BLE 名称
  --verbose, -v         输出下载、缓存、校验和路径细节，便于开发调试
  -h, --help            显示帮助

示例:
  bash <(curl -fsSL https://install.yundrone.cn/ble-server.sh)
  bash <(curl -fsSL https://install.yundrone.cn/ble.sh) -- server
  bash <(curl -fsSL https://install.yundrone.cn/ble-server.sh) -- install --yes
  bash <(curl -fsSL https://install.yundrone.cn/ble-server.sh) -- doctor
  bash <(curl -fsSL https://install.yundrone.cn/ble-server.sh) -- uninstall
EOF
}

parse_args() {
  if [ "${1:-}" = "--" ]; then
    shift
  fi

  if [ $# -gt 0 ]; then
    case "$1" in
      menu|install|update|uninstall|reinstall|status|doctor|logs|reset-name)
        COMMAND="$1"
        shift
        ;;
    esac
  fi

  while [ $# -gt 0 ]; do
    case "$1" in
      --prefix)
        PREFIX="${2:-}"
        [ -n "$PREFIX" ] || fail "--prefix 需要一个值"
        shift 2
        ;;
      --version)
        VERSION="${2:-}"
        [ -n "$VERSION" ] || fail "--version 需要一个值"
        shift 2
        ;;
      --adapter)
        ADAPTER="${2:-}"
        [ -n "$ADAPTER" ] || fail "--adapter 需要一个值"
        shift 2
        ;;
      --backend)
        BACKEND="${2:-}"
        [ -n "$BACKEND" ] || fail "--backend 需要一个值"
        shift 2
        ;;
      --yes|-y)
        ASSUME_YES="yes"
        shift
        ;;
      --purge)
        PURGE="yes"
        shift
        ;;
      --reset-name)
        RESET_NAME="yes"
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

validate_prefix() {
  case "$PREFIX" in
    ""|*-|"-"*|*--*)
      fail "非法前缀 '$PREFIX'：只能使用小写字母、数字和单个 '-' 分隔"
      ;;
  esac
  if ! printf '%s' "$PREFIX" | grep -Eq '^[a-z0-9][a-z0-9-]*[a-z0-9]$|^[a-z0-9]$'; then
    fail "非法前缀 '$PREFIX'：只能使用小写字母、数字和 '-'"
  fi
}

main() {
  parse_args "$@"
  if use_tui; then
    case "$COMMAND" in
      menu|install|update|uninstall|reinstall|logs|reset-name)
        ensure_gum
        ;;
    esac
  fi
  case "$COMMAND" in
    menu)
      menu
      ;;
    install|update)
      install_or_update
      ;;
    uninstall)
      uninstall
      ;;
    reinstall)
      uninstall
      install_or_update
      ;;
    status)
      render_summary
      ;;
    doctor)
      doctor
      ;;
    logs)
      show_logs
      ;;
    reset-name)
      reset_name
      ;;
    *)
      fail "未知命令: $COMMAND"
      ;;
  esac
}

main "$@"
