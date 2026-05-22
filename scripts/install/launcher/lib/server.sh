controlled_status_label() {
  if ! is_linux; then
    printf '%s' "$(tr_text "本机不支持被控端" "Controlled device not supported locally")"
    return 0
  fi
  if have systemctl && systemctl is-active --quiet yundrone-ble-command-gateway.service 2>/dev/null; then
    printf '%s' "$(tr_text "运行中" "Running")"
  elif have systemctl && systemctl list-unit-files yundrone-ble-command-gateway.service >/dev/null 2>&1; then
    printf '%s' "$(tr_text "已安装但未运行" "Installed but not running")"
  else
    printf '%s' "$(tr_text "未安装" "Not installed")"
  fi
}

server_status_label() {
  controlled_status_label
}

controlled_delegate() {
  local command="${1:-}"
  if ! is_linux; then
    if use_tui; then
      tui_card "$(tr_text "当前系统不是 Linux，不能在本机部署被控端。\n\n你可以在这台电脑上启动客户端，然后连接一台运行被控端服务的 Linux 边缘设备。" "This system is not Linux, so it cannot run the controlled-device service locally.\n\nYou can launch the client on this computer and connect to a Linux edge device running the controlled-device service.")" 214
      tui_pause
      return 0
    fi
    fail "$(tr_text "被控端只支持部署到 Linux 设备" "The controlled-device service can only be deployed on Linux")"
  fi

  local args=()
  [ -n "$command" ] && args+=("$command")
  [ "$ASSUME_YES" = "yes" ] && args+=("--yes")
  [ "$VERBOSE" = "yes" ] && args+=("--verbose")
  info "$(tr_text "打开被控端安装器" "Opening controlled-device installer")"
  have bash || fail "$(tr_text "需要 bash 启动被控端安装器" "bash is required to start the controlled-device installer")"
  local tmp
  tmp="$(mktemp)"
  fetch "$SERVER_ENTRY_URL" "$tmp"
  bash "$tmp" "${args[@]}"
  rm -f "$tmp"
}

server_delegate() {
  controlled_delegate "$@"
}

controlled_menu() {
  if ! use_tui; then
    controlled_delegate
    return 0
  fi

  ensure_gum
  local choice
  while true; do
    tui_clear
    tui_title
    tui_card "$(printf '%s\n\n%s：%s\n%s：%s\n%s：%s\n\n%s' \
      "$(tr_text "被控端" "Controlled device")" \
      "$(tr_text "状态" "Status")" "$(controlled_status_label)" \
      "$(tr_text "平台" "Platform")" "$(detect_platform)" \
      "$(tr_text "入口" "Entry")" "$SERVER_ENTRY_URL" \
      "$(tr_text "被控端部署在 Linux 边缘设备上，负责广播 yundrone-* 并执行 Wi-Fi 配网/诊断命令。" "The controlled-device service runs on a Linux edge device, advertises yundrone-*, and executes Wi-Fi provisioning/diagnostic commands.")")" 39
    choice="$(printf '%s\n' \
      "$(tr_text "打开被控端安装向导" "Open controlled-device installer")" \
      "$(tr_text "运行被控端诊断" "Run controlled-device diagnostics")" \
      "$(tr_text "返回" "Back")" | tui_choose "$(tr_text "被控端操作" "Controlled-device actions")")" || return 0
    case "$choice" in
      "$(tr_text "打开被控端安装向导" "Open controlled-device installer")") controlled_delegate ;;
      "$(tr_text "运行被控端诊断" "Run controlled-device diagnostics")") controlled_delegate doctor ;;
      "$(tr_text "返回" "Back")") return 0 ;;
    esac
  done
}

server_menu() {
  controlled_menu
}
