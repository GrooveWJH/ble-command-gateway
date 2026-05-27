GUM_VERSION="${YUNDRONE_GUM_VERSION:-0.17.0}"
GUM_BIN=""

gum_platform() {
  local platform
  platform="$(detect_platform)"
  case "$platform" in
    linux-amd64|linux-arm64|macos-arm64)
      printf '%s' "$platform"
      ;;
    *)
      printf '%s' "unsupported"
      ;;
  esac
}

gum_bin_path() {
  local platform="$1"
  printf '%s/%s/%s/gum' "$GUM_CACHE_ROOT" "$GUM_VERSION" "$platform"
}

gum_download_and_install() {
  local platform="$1"
  local tmp metadata url expected actual tarball target_dir found
  tmp="$(mktemp -d)"
  metadata="${tmp}/gum-latest.json"
  debug "gum metadata=${TOOLS_BASE_URL}/gum/latest.json"
  fetch "${TOOLS_BASE_URL}/gum/latest.json" "$metadata"
  GUM_VERSION="$(json_value "version" "$metadata")"
  url="$(json_asset_value "url" "$platform" "$metadata")"
  expected="$(json_asset_value "sha256" "$platform" "$metadata")"
  tarball="${tmp}/gum.tar.gz"

  info "下载 gum TUI 引擎：$(version_text "$GUM_VERSION") / $(accent "$platform")"
  debug "gum url=${url}"
  fetch "$url" "$tarball" progress
  actual="$(sha256_file "$tarball")"
  debug "gum sha256 expected=${expected} actual=${actual}"
  [ "$actual" = "$expected" ] || fail "gum 校验失败：${platform}"

  target_dir="${GUM_CACHE_ROOT}/${GUM_VERSION}/${platform}"
  rm -rf "${tmp}/extract" "$target_dir"
  mkdir -p "${tmp}/extract" "$target_dir"
  tar -xzf "$tarball" -C "${tmp}/extract"
  found="$(find "${tmp}/extract" -type f -name gum | head -n 1)"
  [ -n "$found" ] || fail "gum 安装包中没有 gum 可执行文件"
  cp "$found" "${target_dir}/gum"
  chmod +x "${target_dir}/gum"
  rm -rf "$tmp"
}

ensure_gum() {
  local platform
  platform="$(gum_platform)"
  [ "$platform" != "unsupported" ] || fail "当前架构不支持 gum TUI：$(uname -s) $(uname -m)。"

  GUM_BIN="$(gum_bin_path "$platform")"
  if [ -x "$GUM_BIN" ]; then
    debug "gum cache hit=${GUM_BIN}"
    return 0
  fi

  gum_download_and_install "$platform"
  GUM_BIN="$(gum_bin_path "$platform")"
  [ -x "$GUM_BIN" ] || fail "gum TUI 引擎安装失败"
}

gum() {
  TERM="${TERM:-xterm-256color}" \
    COLORTERM="${COLORTERM:-truecolor}" \
    COLORFGBG="${COLORFGBG:-15;0}" \
    "$GUM_BIN" "$@"
}

tui_clear() {
  printf '\033[2J\033[H'
}

tui_title() {
  local subtitle
  subtitle="$(tr_text "启动客户端，或部署 Linux 被控端" "Launch the client, or deploy a Linux controlled device")"
  gum style \
    --foreground 15 \
    --background 24 \
    --bold \
    --width 76 \
    --padding "0 2" \
    --margin "1 0 0 2" \
    "YunDrone BLE Wi-Fi Tool"
  gum style \
    --foreground 245 \
    --width 76 \
    --margin "0 0 1 2" \
    "$subtitle"
}

tui_card() {
  gum style \
    --border rounded \
    --border-foreground "${2:-39}" \
    --width "${TUI_WIDTH:-72}" \
    --padding "1 2" \
    --margin "0 0 1 2" \
    "$1"
}

tui_choose() {
  tui_choose_raw "$@"
}

tui_choose_raw() {
  gum choose \
    --cursor "▸ " \
    --cursor.foreground 212 \
    --selected.foreground 15 \
    --header.foreground 39 \
    --item.foreground 250 \
    --height "${TUI_MENU_HEIGHT:-8}" \
    --padding "0 0 0 2" \
    --header "$1"
}

tui_confirm() {
  gum confirm \
    --affirmative "$(tr_text "继续" "Continue")" \
    --negative "$(tr_text "取消" "Cancel")" \
    --prompt.foreground 214 \
    "$1"
}

tui_spin() {
  local title="$1"
  shift
  gum spin \
    --spinner dot \
    --spinner.foreground 39 \
    --title.foreground 212 \
    --title "$title" \
    -- "$@"
}

tui_pause() {
  gum input --placeholder "$(tr_text "按回车返回" "Press Enter to return")" >/dev/null || true
}
