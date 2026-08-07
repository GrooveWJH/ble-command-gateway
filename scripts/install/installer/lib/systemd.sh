write_service() {
  local prefix="$1"
  local backend="$2"
  local content
  content="$(cat <<EOF
[Unit]
Description=YunDrone BLE Command Gateway Server
After=bluetooth.service NetworkManager.service
Wants=bluetooth.service NetworkManager.service

[Service]
Type=simple
User=root
WorkingDirectory=${INSTALL_ROOT}/current
Environment=YUNDRONE_BLE_ADV_BACKEND=${backend}
Environment=YUNDRONE_LOG_COLOR=always
Environment=YUNDRONE_DEVICE_PREFIX=${prefix}
ExecStartPre=${INSTALL_ROOT}/current/deploy/systemd/prepare-ble-adapter.sh
ExecStart=${INSTALL_ROOT}/current/yundrone-ble-server
Restart=on-failure
RestartSec=5
TimeoutStartSec=45

[Install]
WantedBy=multi-user.target
EOF
)"
  printf '%s\n' "$content" | run_root tee "$SERVICE_PATH" >/dev/null
}

print_service_diagnostics() {
  warn "服务启动没有按预期完成，下面是诊断信息。"
  printf '\n%s\n' "$(danger_text "== systemctl status ==")"
  run_root systemctl status "$SERVICE_NAME" --no-pager || true
  printf '\n%s\n' "$(danger_text "== recent logs ==")"
  run_root journalctl -u "$SERVICE_NAME" -n "$LOG_LINES" -o cat --no-pager || true
  printf '\n%s\n' "$(warn_text "你也可以手动查看实时日志：")"
  printf '  %s\n' "$(command_text "sudo journalctl -u ${SERVICE_NAME} -f -o cat")"
}

enable_service() {
  info "启用服务"
  if ! run_root systemctl enable "$SERVICE_NAME" >/dev/null 2>&1; then
    print_service_diagnostics
    fail "服务设置为开机自启失败"
  fi
}

start_or_restart_service() {
  local action
  if systemctl is-active "$SERVICE_NAME" >/dev/null 2>&1; then
    action="restart"
    info "重启服务"
  else
    action="start"
    info "启动服务"
  fi

  if ! run_root_with_timeout "$SERVICE_START_TIMEOUT" systemctl "$action" "$SERVICE_NAME" >/dev/null 2>&1; then
    if systemctl is-active "$SERVICE_NAME" >/dev/null 2>&1; then
      warn "systemctl ${action} 返回异常，但服务当前已处于运行状态。"
      return 0
    fi
    print_service_diagnostics
    fail "服务${action}超时或失败"
  fi
}

wait_service_active() {
  local waited=0
  while [ "$waited" -lt "$SERVICE_START_TIMEOUT" ]; do
    if systemctl is-active "$SERVICE_NAME" >/dev/null 2>&1; then
      return 0
    fi
    sleep 1
    waited=$((waited + 1))
  done

  print_service_diagnostics
  fail "服务启动后未进入 active 状态"
}

wait_identity_ready() {
  local waited=0
  while [ "$waited" -lt "$SERVICE_START_TIMEOUT" ]; do
    if [ "$(identity_serial)" != "null" ]; then
      return 0
    fi
    if ! systemctl is-active "$SERVICE_NAME" >/dev/null 2>&1; then
      break
    fi
    sleep 1
    waited=$((waited + 1))
  done

  print_service_diagnostics
  fail "等待蓝牙适配器超时；当前诊断身份为 ${PREFIX}-null"
}

start_service_checked() {
  enable_service
  start_or_restart_service
  wait_service_active
}

stop_service_for_install() {
  if ! unit_file_exists; then
    return 0
  fi
  if ! systemctl is-active "$SERVICE_NAME" >/dev/null 2>&1; then
    return 0
  fi

  info "停止旧服务"
  if ! run_root_no_block_systemctl stop "$SERVICE_NAME" >/dev/null 2>&1; then
    print_service_diagnostics
    fail "旧服务停止请求提交失败，已取消替换安装文件"
  fi

  local waited=0
  while [ "$waited" -lt "$SERVICE_START_TIMEOUT" ]; do
    if ! systemctl is-active "$SERVICE_NAME" >/dev/null 2>&1; then
      return 0
    fi
    sleep 1
    waited=$((waited + 1))
  done

  warn "旧服务停止超时，尝试终止残留进程。"
  run_root systemctl kill --kill-who=all --signal=TERM "$SERVICE_NAME" >/dev/null 2>&1 || true
  sleep 2
  if systemctl is-active "$SERVICE_NAME" >/dev/null 2>&1; then
    run_root systemctl kill --kill-who=all --signal=KILL "$SERVICE_NAME" >/dev/null 2>&1 || true
  fi

  waited=0
  while [ "$waited" -lt 10 ]; do
    if ! systemctl is-active "$SERVICE_NAME" >/dev/null 2>&1; then
      return 0
    fi
    sleep 1
    waited=$((waited + 1))
  done

  print_service_diagnostics
  fail "旧服务停止超时或失败，已取消替换安装文件"
}
