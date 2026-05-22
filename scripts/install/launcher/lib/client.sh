client_metadata_path() {
  printf '%s/latest.json' "${CLIENT_CACHE_ROOT}/metadata"
}

client_current_link() {
  printf '%s/current' "$CLIENT_CACHE_ROOT"
}

client_current_bin() {
  printf '%s/yundrone-ble-client' "$(client_current_link)"
}

client_status_label() {
  local bin
  bin="$(client_current_bin)"
  if [ -x "$bin" ]; then
    "$bin" --version 2>/dev/null | head -n 1 || printf '%s' "已安装"
  else
    printf '%s' "未安装"
  fi
}

client_resolve_asset() {
  local platform="$1"
  local tmp="$2"
  local metadata="${tmp}/latest.json"
  local version url expected

  mkdir -p "${CLIENT_CACHE_ROOT}/metadata"
  debug "client metadata=${CLIENT_BASE_URL}/releases/latest.json"
  fetch "${CLIENT_BASE_URL}/releases/latest.json" "$metadata"
  version="$(json_value "latest" "$metadata" 2>/dev/null || json_value "version" "$metadata")"
  url="$(json_asset_value "url" "$platform" "$metadata" 2>/dev/null || true)"
  expected="$(json_asset_value "sha256" "$platform" "$metadata" 2>/dev/null || true)"
  [ -n "$url" ] && [ -n "$expected" ] || fail "安装源没有提供 ${platform} 的 client 包"

  cp "$metadata" "$(client_metadata_path)"
  printf '%s\n' "$version" >"${tmp}/version"
  printf '%s\n' "$url" >"${tmp}/url"
  printf '%s\n' "$expected" >"${tmp}/sha256"
}

client_install_or_update() {
  local platform tmp version url expected tarball actual target_dir found
  platform="$(detect_platform)"
  case "$platform" in
    macos-arm64|linux-amd64|linux-arm64) ;;
    *) fail "当前平台不支持 client 预编译包：$(uname -s) $(uname -m)" ;;
  esac

  have python3 || fail "需要 python3 解析 client release metadata"
  have tar || fail "需要 tar 解包 client"
  have_sha256 || fail "需要 sha256sum 或 shasum 校验 client"
  { have curl || have wget; } || fail "需要 curl 或 wget 下载 client"

  tmp="$(mktemp -d)"
  client_resolve_asset "$platform" "$tmp"
  version="$(cat "${tmp}/version")"
  url="$(cat "${tmp}/url")"
  expected="$(cat "${tmp}/sha256")"
  tarball="${tmp}/client.tar.gz"
  target_dir="${CLIENT_CACHE_ROOT}/versions/${version}/${platform}"

  if [ -x "${target_dir}/yundrone-ble-client" ]; then
    debug "client cache hit=${target_dir}/yundrone-ble-client"
    ln -sfn "$target_dir" "$(client_current_link)"
    rm -rf "$tmp"
    return 0
  fi

  info "下载 YunDrone BLE Client：$(version_text "$version") / $(accent "$platform")"
  debug "client url=${url}"
  fetch "$url" "$tarball" progress
  actual="$(sha256_file "$tarball")"
  debug "client sha256 expected=${expected} actual=${actual}"
  [ "$actual" = "$expected" ] || fail "client 校验失败：${platform}"

  rm -rf "${tmp}/extract" "$target_dir"
  mkdir -p "${tmp}/extract" "$target_dir"
  tar -xzf "$tarball" -C "${tmp}/extract"
  found="$(find "${tmp}/extract" -type f -name yundrone-ble-client | head -n 1)"
  [ -n "$found" ] || fail "client 安装包中没有 yundrone-ble-client"
  cp "$found" "${target_dir}/yundrone-ble-client"
  chmod +x "${target_dir}/yundrone-ble-client"
  printf '%s\n' "$version" >"${target_dir}/VERSION"
  ln -sfn "$target_dir" "$(client_current_link)"
  rm -rf "$tmp"
  ok "client 已就绪：$(version_text "$version")"
}

client_launch() {
  use_tui && ensure_gum
  client_install_or_update

  local bin
  bin="$(client_current_bin)"
  [ -x "$bin" ] || fail "client 未安装成功"
  info "启动 BLE Client CLI"
  exec "$bin" interactive --lang zh
}

client_update_only() {
  client_install_or_update
  if use_tui; then
    gum style --foreground 42 "✓ Client 已更新"
    tui_pause
  fi
}

client_clear_cache() {
  if use_tui; then
    tui_confirm "确认删除本地 client 缓存？" || return 0
  fi
  rm -rf "$CLIENT_CACHE_ROOT"
  ok "已删除 client 缓存"
}

client_menu() {
  ensure_gum
  local choice
  while true; do
    tui_clear
    tui_title
    tui_card "Client CLI

状态：$(client_status_label)
平台：$(detect_platform)
缓存：${CLIENT_CACHE_ROOT}

Client 将以裸二进制方式启动，不使用 macOS .app。" 99
    choice="$(printf '%s\n' \
      "启动交互式 Client" \
      "下载 / 更新 Client" \
      "删除本地 Client 缓存" \
      "返回" | tui_choose "Client 操作")" || return 0
    case "$choice" in
      启动交互式\ Client) client_launch ;;
      "下载 / 更新 Client") client_update_only ;;
      删除本地\ Client\ 缓存) client_clear_cache ;;
      返回) return 0 ;;
    esac
  done
}
