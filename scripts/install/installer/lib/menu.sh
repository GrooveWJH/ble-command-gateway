not_installed_menu() {
  local choice
  while true; do
    tui_ready && tui_clear
    render_summary
    render_recommended_action

    if tui_ready; then
      choice="$(printf '%s\n' \
        "安装或更新 server" \
        "运行环境诊断" \
        "退出" | tui_choose "下一步")" || exit 0
      case "$choice" in
        "安装或更新 server") install_or_update; exit 0 ;;
        运行环境诊断) doctor ;;
        退出) exit 0 ;;
      esac
      continue
    fi

    subheading "可选操作"
    printf '  1) 立即安装\n'
    printf '  2) 运行环境诊断\n'
    printf '  3) 退出\n'
    read_choice_into choice
    case "$choice" in
      1) install_or_update; exit 0 ;;
      2) doctor ;;
      3|q|Q) exit 0 ;;
      *) warn "无效选择" ;;
    esac
  done
}

healthy_menu() {
  local choice
  while true; do
    tui_ready && tui_clear
    render_summary
    render_recommended_action

    if tui_ready; then
      choice="$(printf '%s\n' \
        "查看最近日志" \
        "重启服务" \
        "检查更新 / 更新" \
        "重置 BLE 名称" \
        "运行环境诊断" \
        "卸载" \
        "退出" | tui_choose "维护操作")" || exit 0
      case "$choice" in
        查看最近日志) show_logs ;;
        重启服务) restart_service ;;
        "检查更新 / 更新") install_or_update; exit 0 ;;
        "重置 BLE 名称") reset_name ;;
        运行环境诊断) doctor ;;
        卸载) uninstall; exit 0 ;;
        退出) exit 0 ;;
      esac
      continue
    fi

    subheading "维护操作"
    printf '  1) 查看最近日志\n'
    printf '  2) 重启服务\n'
    printf '  3) 检查更新 / 更新\n'
    printf '  4) 卸载\n'
    printf '  5) 重置 BLE 名称\n'
    printf '  6) 运行环境诊断\n'
    printf '  7) 退出\n'
    read_choice_into choice
    case "$choice" in
      1) show_logs ;;
      2) restart_service ;;
      3) install_or_update; exit 0 ;;
      4) uninstall; exit 0 ;;
      5) reset_name ;;
      6) doctor ;;
      7|q|Q) exit 0 ;;
      *) warn "无效选择" ;;
    esac
  done
}

failed_menu() {
  local choice
  while true; do
    tui_ready && tui_clear
    render_summary
    render_recommended_action

    if tui_ready; then
      choice="$(printf '%s\n' \
        "查看最近日志" \
        "尝试重启服务" \
        "修复安装 / 覆盖更新" \
        "运行环境诊断" \
        "卸载" \
        "退出" | tui_choose "修复操作")" || exit 0
      case "$choice" in
        查看最近日志) show_logs ;;
        尝试重启服务) restart_service ;;
        "修复安装 / 覆盖更新") install_or_update; exit 0 ;;
        运行环境诊断) doctor ;;
        卸载) uninstall; exit 0 ;;
        退出) exit 0 ;;
      esac
      continue
    fi

    subheading "修复操作"
    printf '  1) 查看最近日志\n'
    printf '  2) 尝试重启服务\n'
    printf '  3) 修复安装 / 覆盖更新\n'
    printf '  4) 卸载\n'
    printf '  5) 运行环境诊断\n'
    printf '  6) 退出\n'
    read_choice_into choice
    case "$choice" in
      1) show_logs ;;
      2) restart_service ;;
      3) install_or_update; exit 0 ;;
      4) uninstall; exit 0 ;;
      5) doctor ;;
      6|q|Q) exit 0 ;;
      *) warn "无效选择" ;;
    esac
  done
}

partial_menu() {
  local choice
  while true; do
    tui_ready && tui_clear
    render_summary
    render_recommended_action

    if tui_ready; then
      choice="$(printf '%s\n' \
        "清理并重新安装" \
        "运行环境诊断" \
        "卸载清理" \
        "退出" | tui_choose "修复操作")" || exit 0
      case "$choice" in
        清理并重新安装) PURGE="no"; uninstall; install_or_update; exit 0 ;;
        运行环境诊断) doctor ;;
        卸载清理) uninstall; exit 0 ;;
        退出) exit 0 ;;
      esac
      continue
    fi

    subheading "修复操作"
    printf '  1) 清理并重新安装\n'
    printf '  2) 运行环境诊断\n'
    printf '  3) 卸载清理\n'
    printf '  4) 退出\n'
    read_choice_into choice
    case "$choice" in
      1) PURGE="no"; uninstall; install_or_update; exit 0 ;;
      2) doctor ;;
      3) uninstall; exit 0 ;;
      4|q|Q) exit 0 ;;
      *) warn "无效选择" ;;
    esac
  done
}

menu() {
  case "$(detect_install_state)" in
    not_installed) not_installed_menu ;;
    healthy) healthy_menu ;;
    service_failed) failed_menu ;;
    partial) partial_menu ;;
    *) doctor ;;
  esac
}
