server_status_label() {
  if ! is_linux; then
    printf '%s' "本机不支持 server"
    return 0
  fi
  if have systemctl && systemctl is-active --quiet yundrone-ble-command-gateway.service 2>/dev/null; then
    printf '%s' "运行中"
  elif have systemctl && systemctl list-unit-files yundrone-ble-command-gateway.service >/dev/null 2>&1; then
    printf '%s' "已安装但未运行"
  else
    printf '%s' "未安装"
  fi
}

server_delegate() {
  local command="${1:-}"
  if ! is_linux; then
    if use_tui; then
      tui_card "当前系统不是 Linux，不能在本机部署 BLE Server。

你可以在这台电脑上启动 BLE Client，然后连接一台运行 Server 的 Linux 边缘设备。" 214
      tui_pause
      return 0
    fi
    fail "BLE Server 只支持部署到 Linux 设备"
  fi

  local args=()
  [ -n "$command" ] && args+=("$command")
  [ "$ASSUME_YES" = "yes" ] && args+=("--yes")
  [ "$VERBOSE" = "yes" ] && args+=("--verbose")
  info "打开 BLE Server 安装器"
  have bash || fail "需要 bash 启动 server 安装器"
  local tmp
  tmp="$(mktemp)"
  fetch "$SERVER_ENTRY_URL" "$tmp"
  bash "$tmp" "${args[@]}"
  rm -f "$tmp"
}

server_menu() {
  if ! use_tui; then
    server_delegate
    return 0
  fi

  ensure_gum
  local choice
  while true; do
    tui_clear
    tui_title
    tui_card "BLE Server

状态：$(server_status_label)
平台：$(detect_platform)
入口：${SERVER_ENTRY_URL}

Server 部署在 Linux 边缘设备上，负责广播 yundrone-* 并执行 Wi-Fi 配网/诊断命令。" 39
    choice="$(printf '%s\n' \
      "打开 Server 安装向导" \
      "运行 Server 诊断" \
      "返回" | tui_choose "Server 操作")" || return 0
    case "$choice" in
      打开\ Server\ 安装向导) server_delegate ;;
      运行\ Server\ 诊断) server_delegate doctor ;;
      返回) return 0 ;;
    esac
  done
}
