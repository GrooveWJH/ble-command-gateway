doctor() {
  section "YunDrone BLE Server 环境诊断"
  local arch os_status source_status required_ok warn_count state
  required_ok="yes"
  warn_count=0
  arch="$(safe_arch)"
  state="$(detect_install_state)"

  if [ "$(uname -s)" = "Linux" ]; then
    os_status="OK"
  else
    os_status="不支持"
    required_ok="no"
  fi

  [ "$arch" != "unsupported" ] || required_ok="no"
  have systemctl || required_ok="no"
  if [ "$(id -u)" -ne 0 ] && ! have sudo; then required_ok="no"; fi
  { have curl || have wget; } || required_ok="no"
  have python3 || required_ok="no"
  have tar || required_ok="no"
  { have sha256sum || have shasum; } || required_ok="no"
  have bluetoothctl || required_ok="no"

  if install_source_ok; then
    source_status="OK"
  else
    source_status="不可访问"
    required_ok="no"
  fi

  [ "$(networkmanager_state)" = "active" ] || warn_count=$((warn_count + 1))
  [ "$(adapter_display)" != "未发现" ] || required_ok="no"

  if tui_ready; then
    tui_title "YunDrone BLE Server 环境诊断"
    {
      printf '类别,检查项,结果\n'
      printf '必需项,OS,%s %s\n' "$(uname -s)" "$os_status"
      printf '必需项,架构,%s -> %s\n' "$(uname -m)" "$arch"
      printf '必需项,systemd,%s\n' "$(tool_ok systemctl)"
      printf '必需项,sudo/root,%s\n' "$([ "$(id -u)" -eq 0 ] || have sudo && echo OK || echo 缺失)"
      printf '必需项,curl/wget,%s\n' "$(if have curl || have wget; then echo OK; else echo 缺失; fi)"
      printf '必需项,python3,%s\n' "$(tool_ok python3)"
      printf '必需项,tar,%s\n' "$(tool_ok tar)"
      printf '必需项,sha256sum/shasum,%s\n' "$(if have sha256sum || have shasum; then echo OK; else echo 缺失; fi)"
      printf '必需项,bluetoothctl,%s\n' "$(tool_ok bluetoothctl)"
      printf '必需项,蓝牙适配器,%s\n' "$(adapter_display)"
      printf '必需项,安装源,%s\n' "$source_status"
      printf '推荐项,NetworkManager,%s\n' "$(networkmanager_state)"
      printf '推荐项,当前安装状态,%s\n' "$(state_label "$state")"
    } | gum table --border rounded --separator "," --columns "类别,检查项,结果"

    local conclusion
    if [ "$required_ok" != "yes" ]; then
      conclusion="$(printf '诊断结论\n\n暂不可安装。请先补齐上面的必需项。')"
      tui_danger_card "$conclusion"
    elif [ "$warn_count" -gt 0 ]; then
      conclusion="$(printf '诊断结论\n\n可以安装，但有警告。Wi-Fi 记忆管理依赖 NetworkManager，建议确认它处于 active。')"
      tui_warn_card "$conclusion"
    else
      conclusion="$(printf '诊断结论\n\n可以安装。当前机器满足 YunDrone BLE Server 的基础要求。')"
      tui_success_card "$conclusion"
    fi
    return 0
  fi

  strong "必需项"
  printf '\n'
  field_line "OS" "$(accent "$(uname -s)") $(status_text "$os_status")"
  field_line "架构" "$(muted "$(uname -m)") $(muted "->") $(accent "$arch")"
  field_line "systemd" "$(status_text "$(tool_ok systemctl)")"
  field_line "sudo/root" "$(status_text "$([ "$(id -u)" -eq 0 ] || have sudo && echo OK || echo 缺失)")"
  field_line "curl/wget" "$(status_text "$(if have curl || have wget; then echo OK; else echo 缺失; fi)")"
  field_line "python3" "$(status_text "$(tool_ok python3)")"
  field_line "tar" "$(status_text "$(tool_ok tar)")"
  field_line "sha256sum/shasum" "$(status_text "$(if have sha256sum || have shasum; then echo OK; else echo 缺失; fi)")"
  field_line "bluetoothctl" "$(status_text "$(tool_ok bluetoothctl)")"
  field_line "蓝牙适配器" "$(status_text "$(adapter_display)")"
  field_line "安装源" "$(status_text "$source_status")"

  subheading "推荐项"
  field_line "NetworkManager" "$(status_text "$(networkmanager_state)")"
  field_line "当前安装状态" "$(status_text "$(state_label "$state")")"

  subheading "诊断结论"
  if [ "$required_ok" != "yes" ]; then
    printf '  %s\n' "$(bad_text "暂不可安装。请先补齐上面的必需项。")"
  elif [ "$warn_count" -gt 0 ]; then
    printf '  %s\n' "$(warn_text "可以安装，但有警告。Wi-Fi 记忆管理依赖 NetworkManager，建议确认它处于 active。")"
  else
    printf '  %s\n' "$(good_text "可以安装。当前机器满足 YunDrone BLE Server 的基础要求。")"
  fi
}

prepare_staging_dir() {
  local staging_dir="$1"
  info "写入安装目录"
  run_root mkdir -p "$INSTALL_ROOT/releases"
  run_root rm -rf "$staging_dir"
  run_root mkdir -p "$staging_dir"
}

activate_release() {
  local staging_dir="$1"
  local release_dir="$2"
  run_root rm -rf "$release_dir"
  run_root mv "$staging_dir" "$release_dir"
  run_root ln -sfn "$release_dir" "${INSTALL_ROOT}/current"
}

install_or_update() {
  validate_prefix
  need_sudo
  maybe_install_dependencies
  ensure_basic_tools
  ensure_python

  local arch adapter tmp tarball selected_version release_dir staging_dir
  arch="$(detect_arch)"
  adapter="$(adapter_display)"
  tmp="$(mktemp -d)"
  trap 'rm -rf "$tmp"' EXIT

  section "准备安装"
  strong "安装配置"
  printf '\n'
  field_line "版本" "$(version_text "$VERSION")"
  field_line "架构" "$(accent "$arch")"
  field_line "BLE 名前缀" "$(accent "$PREFIX")"
  field_line "广播后端" "$(accent "$BACKEND")"
  field_line "蓝牙适配器" "$(accent "$adapter")"
  field_line "安装目录" "$(path_text "$INSTALL_ROOT")"
  field_line "systemd service" "$(accent "$SERVICE_NAME")"
  if [ "$RESET_NAME" = "yes" ]; then
    field_line "BLE 名称" "$(warn_text "重新生成")"
  else
    field_line "BLE 名称" "$(good_text "保留已有名称")$(muted "；没有则首次启动时生成")"
  fi
  confirm_yes "是否继续安装？" || fail "已取消安装"

  tui_run_step "解析 server release 信息" resolve_release_asset "$arch" "$tmp"
  download_release_tarball "$tmp"
  tui_run_step "校验 server release" verify_release_tarball "$arch" "$tmp"
  tarball="${tmp}/server.tar.gz"
  selected_version="$(cat "${tmp}/selected-version")"
  release_dir="${INSTALL_ROOT}/releases/${selected_version}"
  staging_dir="${INSTALL_ROOT}/.staging-${selected_version}-$$"

  tui_run_step "准备安装目录" prepare_staging_dir "$staging_dir"
  tui_run_step "解包 server 文件" extract_release "$tarball" "$staging_dir"

  if [ ! -f "${staging_dir}/yundrone-ble-server" ]; then
    warn "安装包内容如下："
    run_root find "$staging_dir" -maxdepth 3 -print || true
    fail "安装包缺少 yundrone-ble-server"
  fi
  run_root chmod +x "${staging_dir}/yundrone-ble-server"
  if [ -f "${staging_dir}/deploy/systemd/prepare-ble-adapter.sh" ]; then
    run_root chmod +x "${staging_dir}/deploy/systemd/prepare-ble-adapter.sh"
  fi

  tui_run_step "停止旧服务" stop_service_for_install
  tui_run_step "切换 release 目录" activate_release "$staging_dir" "$release_dir"

  if [ "$RESET_NAME" = "yes" ]; then
    tui_run_step "删除旧 BLE 名称" run_root rm -f "$IDENTITY_FILE"
  fi

  tui_run_step "写入 systemd service" write_service "$PREFIX" "$BACKEND"
  tui_run_step "刷新 systemd" run_root systemctl daemon-reload

  tui_run_step "启动 YunDrone BLE Server" start_service_checked

  render_install_success "$selected_version"
  if confirm_no "是否查看最近服务日志？"; then
    show_logs
  fi
  rm -rf "$tmp"
  trap - EXIT
}

render_install_success() {
  local selected_version="$1"
  if tui_ready; then
    tui_success_card "安装完成

版本：${selected_version}
BLE 名称：$(identity_name)
服务：${SERVICE_NAME}
状态：运行中

现在可以用 YunDrone client 或小程序扫描 ${PREFIX}-* 设备。"
    gum style --foreground 39 "实时日志：sudo journalctl -u ${SERVICE_NAME} -f -o cat"
    return 0
  fi

  section "安装完成"
  field_line "版本" "$(version_text "$selected_version")"
  field_line "BLE 名称" "$(accent "$(identity_name)")"
  field_line "服务" "$(accent "$SERVICE_NAME")"
  field_line "状态" "$(good_text "运行中")"
  printf '\n%s\n' "你现在可以用 YunDrone client 或小程序扫描 $(accent "${PREFIX}-*") 设备。"
  printf '%s\n' "查看实时日志："
  printf '  %s\n' "$(command_text "sudo journalctl -u ${SERVICE_NAME} -f -o cat")"
}

uninstall() {
  need_sudo
  section "准备卸载"
  if tui_ready; then
    if [ "$PURGE" = "yes" ]; then
      tui_confirm_danger "准备彻底卸载

将删除：
service：${SERVICE_NAME}
程序目录：${INSTALL_ROOT}
状态目录：${STATE_DIR}

注意：这会删除持久化 BLE 名称，下次安装会生成新名字。" || fail "已取消卸载"
    else
      tui_confirm_danger "准备卸载

将删除：
service：${SERVICE_NAME}
程序目录：${INSTALL_ROOT}

将保留：
状态目录：${STATE_DIR}
BLE 名称：$(identity_name)" || fail "已取消卸载"
    fi
  else
  printf '%s\n' "$(warn_text "将删除：")"
  field_line "service" "$(accent "$SERVICE_NAME")"
  field_line "程序目录" "$(path_text "$INSTALL_ROOT")"
  if [ "$PURGE" = "yes" ]; then
    field_line "状态目录" "$(danger_text "$STATE_DIR")$(bad_text "，也会删除")"
  else
    field_line "状态目录" "$(path_text "$STATE_DIR")$(good_text "，会保留")"
    field_line "BLE 名称" "$(accent "$(identity_name)")$(good_text "，会保留")$(muted "，下次安装继续使用")"
  fi
  confirm_yes "是否继续卸载？" || fail "已取消卸载"
  fi

  run_root systemctl disable --now "$SERVICE_NAME" >/dev/null 2>&1 || true
  run_root rm -f "$SERVICE_PATH"
  run_root systemctl daemon-reload
  run_root rm -rf "$INSTALL_ROOT"
  if [ "$PURGE" = "yes" ]; then
    run_root rm -rf "$STATE_DIR"
  fi
  if tui_ready; then
    local result
    if [ "$PURGE" = "yes" ]; then
      result="$(printf '卸载完成\n\n服务、程序文件和设备状态已删除。')"
      tui_warn_card "$result"
    else
      result="$(printf '卸载完成\n\n服务和程序文件已删除。\nBLE 名称已保留，重新安装后会继续使用同一个设备名。')"
      tui_success_card "$result"
    fi
    return 0
  fi
  section "卸载完成"
  printf '%s\n' "$(good_text "服务和程序文件已删除。")"
  if [ "$PURGE" = "yes" ]; then
    printf '%s\n' "$(warn_text "设备状态也已删除。")"
  else
    printf '%s\n' "$(good_text "BLE 名称已保留，重新安装后会继续使用同一个设备名。")"
  fi
}

reset_name() {
  need_sudo
  if tui_ready; then
    tui_confirm_danger "重置 BLE 名称

将删除：${IDENTITY_FILE}
随后重启服务并生成新的 BLE 名称。" || fail "已取消重置"
  else
    confirm_yes "是否删除 ${IDENTITY_FILE} 并重启服务？" || fail "已取消重置"
  fi
  run_root rm -f "$IDENTITY_FILE"
  run_root systemctl restart "$SERVICE_NAME"
  sleep 2
  if tui_ready; then
    tui_success_card "新的 BLE 名称：$(identity_name)"
  else
    ok "新的 BLE 名称：$(identity_name)"
  fi
}

restart_service() {
  need_sudo
  info "重启 ${SERVICE_NAME}"
  run_root systemctl restart "$SERVICE_NAME"
  sleep 2
  if systemctl is-active "$SERVICE_NAME" >/dev/null 2>&1; then
    ok "服务已重新启动"
  else
    warn "服务重启失败，最近日志如下："
    run_root journalctl -u "$SERVICE_NAME" -n "$LOG_LINES" -o cat --no-pager || true
  fi
}

show_logs() {
  need_sudo
  if tui_ready; then
    tui_info_card "最近 ${LOG_LINES} 行服务日志"
  fi
  run_root journalctl -u "$SERVICE_NAME" -n "$LOG_LINES" -o cat --no-pager
}
