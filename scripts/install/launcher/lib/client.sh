client_metadata_path() {
  printf '%s/latest.json' "${CLIENT_CACHE_ROOT}/metadata"
}

client_current_link() {
  printf '%s/current' "$CLIENT_CACHE_ROOT"
}

client_current_bin() {
  printf '%s/yundrone-ble-client' "$(client_current_link)"
}

client_current_version() {
  local current
  current="$(client_current_link)"
  if [ -f "${current}/VERSION" ]; then
    tr -d '\r\n' <"${current}/VERSION"
  else
    printf '%s' ""
  fi
}

client_current_sha256() {
  local current
  current="$(client_current_link)"
  if [ -f "${current}/SHA256" ]; then
    tr -d '\r\n' <"${current}/SHA256"
  else
    printf '%s' ""
  fi
}

client_status_label() {
  local bin version
  bin="$(client_current_bin)"
  version="$(client_current_version)"
  if [ -x "$bin" ]; then
    if [ -n "$version" ]; then
      printf '%s' "$(tr_text "已安装" "Installed") ${version}"
    else
      printf '%s' "$(tr_text "已安装" "Installed")"
    fi
  else
    printf '%s' "$(tr_text "未安装" "Not installed")"
  fi
}

client_verify_binary() {
  local bin="$1"
  [ -x "$bin" ] || fail "$(tr_text "客户端未安装成功" "Client was not installed successfully")"
  if ! "$bin" --version >/dev/null 2>"${bin}.compat-error"; then
    local detail
    detail="$(cat "${bin}.compat-error" 2>/dev/null || true)"
    rm -f "${bin}.compat-error"
    fail "$(tr_text "客户端二进制无法在当前系统运行。请重新运行一键安装器获取兼容包；如果仍失败，请把下面的错误发给开发者。" "The client binary cannot run on this system. Re-run the installer to fetch a compatible package; if it still fails, send the error below to the developer.")\n${detail}"
  fi
  rm -f "${bin}.compat-error"
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
  [ -n "$url" ] && [ -n "$expected" ] || fail "$(tr_text "安装源没有提供 ${platform} 的客户端包" "The install source does not provide a client package for ${platform}")"

  cp "$metadata" "$(client_metadata_path)"
  printf '%s\n' "$version" >"${tmp}/version"
  printf '%s\n' "$url" >"${tmp}/url"
  printf '%s\n' "$expected" >"${tmp}/sha256"
}

client_install_or_update() {
  local platform tmp version url expected tarball actual target_dir found current_version current_sha target_sha_file
  platform="$(detect_platform)"
  case "$platform" in
    macos-arm64|linux-amd64|linux-arm64) ;;
    *) fail "$(tr_text "当前平台不支持客户端预编译包" "This platform is not supported by the prebuilt client")：$(uname -s) $(uname -m)" ;;
  esac

  have python3 || fail "$(tr_text "需要 python3 解析客户端版本信息" "python3 is required to parse client release metadata")"
  have tar || fail "$(tr_text "需要 tar 解包客户端" "tar is required to extract the client")"
  have_sha256 || fail "$(tr_text "需要 sha256sum 或 shasum 校验客户端" "sha256sum or shasum is required to verify the client")"
  { have curl || have wget; } || fail "$(tr_text "需要 curl 或 wget 下载客户端" "curl or wget is required to download the client")"

  tmp="$(mktemp -d)"
  client_resolve_asset "$platform" "$tmp"
  version="$(cat "${tmp}/version")"
  url="$(cat "${tmp}/url")"
  expected="$(cat "${tmp}/sha256")"
  tarball="${tmp}/client.tar.gz"
  target_dir="${CLIENT_CACHE_ROOT}/versions/${version}/${platform}"
  current_version="$(client_current_version)"
  current_sha="$(client_current_sha256)"
  target_sha_file="${target_dir}/SHA256"

  if [ -x "${target_dir}/yundrone-ble-client" ] && [ -f "$target_sha_file" ] && [ "$(tr -d '\r\n' <"$target_sha_file")" = "$expected" ]; then
    debug "client cache hit=${target_dir}/yundrone-ble-client"
    ln -sfn "$target_dir" "$(client_current_link)"
    rm -rf "$tmp"
    client_verify_binary "${target_dir}/yundrone-ble-client"
    if [ "$current_version" = "$version" ] && [ "$current_sha" = "$expected" ]; then
      ok "$(tr_text "客户端已是最新版本" "Client is already up to date")：$(version_text "$version")"
    else
      ok "$(tr_text "客户端已切换到最新版本" "Client switched to latest version")：$(version_text "$version")"
    fi
    return 0
  fi

  if [ -x "${target_dir}/yundrone-ble-client" ]; then
    info "$(tr_text "检测到同版本客户端包已更新，正在刷新缓存" "Client package changed for the same version, refreshing cache")：$(version_text "$version")"
  elif [ -n "$current_version" ]; then
    info "$(tr_text "发现新版客户端，正在升级" "New client version found, upgrading")：$(version_text "$current_version") -> $(version_text "$version")"
  else
    info "$(tr_text "未检测到客户端，正在安装" "Client not found, installing")：$(version_text "$version")"
  fi
  debug "client url=${url}"
  fetch "$url" "$tarball" progress
  actual="$(sha256_file "$tarball")"
  debug "client sha256 expected=${expected} actual=${actual}"
  [ "$actual" = "$expected" ] || fail "$(tr_text "客户端校验失败" "Client checksum verification failed")：${platform}"

  rm -rf "${tmp}/extract" "$target_dir"
  mkdir -p "${tmp}/extract" "$target_dir"
  tar -xzf "$tarball" -C "${tmp}/extract"
  found="$(find "${tmp}/extract" -type f -name yundrone-ble-client | head -n 1)"
  [ -n "$found" ] || fail "$(tr_text "客户端安装包中没有 yundrone-ble-client" "The client package does not contain yundrone-ble-client")"
  cp "$found" "${target_dir}/yundrone-ble-client"
  chmod +x "${target_dir}/yundrone-ble-client"
  printf '%s\n' "$version" >"${target_dir}/VERSION"
  printf '%s\n' "$expected" >"${target_dir}/SHA256"
  client_verify_binary "${target_dir}/yundrone-ble-client"
  ln -sfn "$target_dir" "$(client_current_link)"
  rm -rf "$tmp"
  ok "$(tr_text "客户端已就绪" "Client is ready")：$(version_text "$version")"
}

client_launch() {
  use_tui && ensure_gum
  client_install_or_update

  local bin
  bin="$(client_current_bin)"
  [ -x "$bin" ] || fail "$(tr_text "客户端未安装成功" "Client was not installed successfully")"
  info "$(tr_text "启动交互式客户端" "Launching interactive client")"
  exec "$bin" interactive --lang "${UI_LANG:-zh}"
}

client_update_only() {
  client_install_or_update
  if use_tui; then
    gum style --foreground 42 "✓ $(tr_text "客户端已更新" "Client updated")"
    tui_pause
  fi
}

client_clear_cache() {
  if use_tui; then
    tui_confirm "$(tr_text "确认删除本地客户端缓存？" "Delete the local client cache?")" || return 0
  fi
  rm -rf "$CLIENT_CACHE_ROOT"
  ok "$(tr_text "已删除客户端缓存" "Client cache removed")"
}

client_menu() {
  ensure_gum
  local choice
  while true; do
    tui_clear
    tui_title
    tui_card "$(printf '%s\n\n%s：%s\n%s：%s\n%s：%s\n\n%s' \
      "$(tr_text "客户端 CLI" "Client CLI")" \
      "$(tr_text "状态" "Status")" "$(client_status_label)" \
      "$(tr_text "平台" "Platform")" "$(detect_platform)" \
      "$(tr_text "缓存" "Cache")" "$CLIENT_CACHE_ROOT" \
      "$(tr_text "启动前会自动检查最新版本；未安装或不是最新时会自动安装/升级。macOS 使用裸二进制，不使用 .app。" "Before launch, the tool checks the latest version and installs/upgrades automatically. macOS uses a raw binary, not an .app bundle.")")" 99
    choice="$(printf '%s\n' \
      "$(tr_text "启动交互式客户端" "Launch interactive client")" \
      "$(tr_text "检查并更新客户端" "Check and update client")" \
      "$(tr_text "删除本地客户端缓存" "Delete local client cache")" \
      "$(tr_text "返回" "Back")" | tui_choose "$(tr_text "客户端操作" "Client actions")")" || return 0
    case "$choice" in
      "$(tr_text "启动交互式客户端" "Launch interactive client")") client_launch ;;
      "$(tr_text "检查并更新客户端" "Check and update client")") client_update_only ;;
      "$(tr_text "删除本地客户端缓存" "Delete local client cache")") client_clear_cache ;;
      "$(tr_text "返回" "Back")") return 0 ;;
    esac
  done
}
