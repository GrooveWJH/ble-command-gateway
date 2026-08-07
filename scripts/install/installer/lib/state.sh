networkmanager_state() {
  if ! have systemctl; then
    printf '%s' "未知"
  else
    systemctl is-active NetworkManager.service 2>/dev/null || printf '%s' "未运行"
  fi
}

install_source_ok() {
  if have curl; then
    curl -fsSL "${INSTALL_BASE_URL}/latest.json" >/dev/null 2>&1
  elif have wget; then
    wget -qO /dev/null "${INSTALL_BASE_URL}/latest.json" >/dev/null 2>&1
  else
    return 1
  fi
}

default_adapter_name() {
  local hci
  if [ -e "${BLUETOOTH_CLASS_DIR}/hci0" ]; then
    printf '%s' "hci0"
    return 0
  fi

  hci="$(
    for candidate in "${BLUETOOTH_CLASS_DIR}"/hci*; do
      [ -e "$candidate" ] || continue
      basename "$candidate"
    done | LC_ALL=C sort | head -n 1
  )"
  [ -n "$hci" ] || return 1
  printf '%s' "$hci"
}

normalize_adapter_address() {
  local compact
  compact="$(printf '%s' "$1" | tr '[:upper:]' '[:lower:]' | tr -d ':-')"
  printf '%s' "$compact" | grep -Eq '^[0-9a-f]{12}$' || return 1
  [ "$compact" != "000000000000" ] || return 1
  [ "$compact" != "ffffffffffff" ] || return 1
  printf '%s' "$compact"
}

default_adapter_address() {
  local hci address first_mac
  hci="$(default_adapter_name 2>/dev/null || true)"
  if [ -n "$hci" ]; then
    address="$(cat "${BLUETOOTH_CLASS_DIR}/${hci}/address" 2>/dev/null || true)"
    if normalize_adapter_address "$address" >/dev/null; then
      printf '%s' "$address"
      return 0
    fi
    return 1
  fi

  if have bluetoothctl; then
    first_mac="$(bluetoothctl list 2>/dev/null | awk 'NR==1 {print $2}')"
    if normalize_adapter_address "$first_mac" >/dev/null; then
      printf '%s' "$first_mac"
      return 0
    fi
  fi

  return 1
}

identity_serial() {
  local address compact
  address="$(default_adapter_address 2>/dev/null || true)"
  compact="$(normalize_adapter_address "$address" 2>/dev/null || true)"
  if [ -z "$compact" ]; then
    printf '%s' "null"
    return 0
  fi
  printf '%s' "${compact#??????}"
}

adapter_display() {
  if [ -n "$ADAPTER" ]; then
    printf '%s' "$ADAPTER"
    return 0
  fi

  local hci address
  hci="$(default_adapter_name 2>/dev/null || true)"
  address="$(default_adapter_address 2>/dev/null || true)"
  if [ -n "$address" ]; then
    printf '%s (%s)' "${hci:-自动检测}" "$address"
    return 0
  fi

  printf '%s' "未发现"
}

adapter_short_display() {
  local display
  display="$(adapter_display)"
  case "$display" in
    自动检测\ *\ \(*)
      printf '%s' "${display%% (*}"
      ;;
    *)
      printf '%s' "$display"
      ;;
  esac
}

unit_file_exists() {
  [ -f "$SERVICE_PATH" ] && return 0
  have systemctl || return 1
  systemctl list-unit-files "$SERVICE_NAME" --no-legend 2>/dev/null | grep -q "^${SERVICE_NAME}[[:space:]]"
}

service_state() {
  if ! have systemctl; then
    printf '%s' "未知"
    return 0
  fi
  if ! unit_file_exists; then
    printf '%s' "未安装"
    return 0
  fi
  systemctl is-active "$SERVICE_NAME" 2>/dev/null || printf '%s' "inactive"
}

installed_version() {
  if [ -f "${INSTALL_ROOT}/current/VERSION" ]; then
    tr -d '\r\n' <"${INSTALL_ROOT}/current/VERSION"
  else
    printf '%s' "未安装"
  fi
}

identity_name() {
  printf '%s-%s' "$PREFIX" "$(identity_serial)"
}

is_binary_installed() {
  [ -x "${INSTALL_ROOT}/current/yundrone-ble-server" ]
}

detect_install_state() {
  local active unit install_dir binary
  active="$(service_state)"
  unit="no"
  install_dir="no"
  binary="no"
  unit_file_exists && unit="yes"
  [ -d "$INSTALL_ROOT" ] && install_dir="yes"
  is_binary_installed && binary="yes"

  if [ "$unit" = "no" ] && [ "$install_dir" = "no" ]; then
    printf '%s' "not_installed"
  elif [ "$active" = "active" ] && [ "$binary" = "yes" ]; then
    printf '%s' "healthy"
  elif [ "$unit" = "yes" ] && [ "$binary" = "yes" ]; then
    printf '%s' "service_failed"
  elif [ "$unit" = "yes" ] || [ "$install_dir" = "yes" ]; then
    printf '%s' "partial"
  else
    printf '%s' "unknown"
  fi
}

state_label() {
  case "$1" in
    not_installed) printf '%s' "未安装" ;;
    healthy) printf '%s' "已安装，运行正常" ;;
    service_failed) printf '%s' "已安装，但服务未运行" ;;
    partial) printf '%s' "存在半安装残留" ;;
    *) printf '%s' "未知" ;;
  esac
}

render_summary() {
  local state arch service
  state="$(detect_install_state)"
  arch="$(safe_arch)"
  service="$(service_state)"
  if tui_ready; then
    local body
    body="$(cat <<EOF
$(gum style --foreground 39 --bold "当前状态")

状态       $(state_label "$state")
服务       ${service}
版本       $(installed_version)
BLE        $(identity_name)
SN         $(identity_serial)
架构       ${arch}
蓝牙       $(adapter_short_display)
EOF
)"
    tui_title "YunDrone BLE Server"
    case "$state" in
      healthy) tui_success_card "$body" ;;
      service_failed) tui_danger_card "$body" ;;
      partial) tui_warn_card "$body" ;;
      *) tui_info_card "$body" ;;
    esac
    return 0
  fi

  section "YunDrone BLE Server"
  strong "当前状态"
  printf '\n'
  field_line "安装状态" "$(status_text "$(state_label "$state")")"
  field_line "服务状态" "$(status_text "$service")"
  field_line "版本" "$(status_text "$(installed_version)")"
  field_line "BLE 名称" "$(status_text "$(identity_name)")"
  field_line "设备 SN" "$(status_text "$(identity_serial)")"
  field_line "系统架构" "$(muted "$(uname -m)") $(muted "->") $(status_text "$arch")"
  field_line "蓝牙适配器" "$(status_text "$(adapter_display)")"
  field_line "安装目录" "$(path_text "$INSTALL_ROOT")"
  field_line "日志命令" "$(command_text "sudo journalctl -u ${SERVICE_NAME} -f -o cat")"
}

render_recommended_action() {
  local state
  state="$(detect_install_state)"
  if tui_ready; then
    local body
    case "$state" in
      not_installed)
        body="$(printf '%s  %s' "$(gum style --foreground 214 --bold "推荐")" "当前未安装，建议现在安装。")"
        tui_warn_card "$body"
        ;;
      healthy)
        body="$(printf '%s  %s' "$(gum style --foreground 42 --bold "状态良好")" "服务正在运行；通常无需操作。")"
        tui_success_card "$body"
        ;;
      service_failed)
        body="$(printf '%s  %s' "$(gum style --foreground 196 --bold "需要处理")" "服务未运行，建议先查看日志或重启。")"
        tui_danger_card "$body"
        ;;
      partial)
        body="$(printf '%s  %s' "$(gum style --foreground 214 --bold "安装残留")" "建议清理并重新安装。")"
        tui_warn_card "$body"
        ;;
      *)
        body="$(printf '%s  %s' "$(gum style --foreground 214 --bold "状态未知")" "建议先运行环境诊断。")"
        tui_warn_card "$body"
        ;;
    esac
    return 0
  fi

  subheading "推荐操作"
  case "$state" in
    not_installed)
      printf '  %s %s\n' "$(warn_text "当前未安装")" "，建议现在安装 YunDrone BLE Server。"
      ;;
    healthy)
      printf '  %s %s\n' "$(good_text "服务正在运行")" "。通常无需操作；如刚发布新版本，可选择检查更新。"
      ;;
    service_failed)
      printf '  %s %s\n' "$(bad_text "程序已安装但服务未运行")" "，建议先查看日志或尝试重启服务。"
      ;;
    partial)
      printf '  %s %s\n' "$(warn_text "检测到安装残留")" "，建议清理并重新安装。"
      ;;
    *)
      printf '  %s %s\n' "$(warn_text "状态不明确")" "，建议先运行环境诊断。"
      ;;
  esac
}
