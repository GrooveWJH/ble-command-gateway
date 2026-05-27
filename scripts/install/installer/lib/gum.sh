GUM_VERSION="${YUNDRONE_GUM_VERSION:-0.17.0}"
GUM_ROOT="${YUNDRONE_GUM_CACHE_ROOT:-${HOME}/.yundrone/ble-launcher/tools/gum}"
GUM_BIN=""

gum_arch() {
  case "$(uname -s):$(uname -m)" in
    Linux:x86_64|Linux:amd64)
      printf '%s' "linux-amd64"
      ;;
    Linux:aarch64|Linux:arm64)
      printf '%s' "linux-arm64"
      ;;
    Darwin:arm64|Darwin:aarch64)
      printf '%s' "macos-arm64"
      ;;
    *)
      printf '%s' "unsupported"
      ;;
  esac
}

gum_json_asset_value() {
  local key="$1"
  local arch="$2"
  local file="$3"
  python3 - "$key" "$arch" "$file" <<'PY'
import json
import sys

key, arch, path = sys.argv[1:]
with open(path, "r", encoding="utf-8") as fh:
    data = json.load(fh)
value = data["assets"][arch]
for part in key.split("."):
    value = value[part]
print(value)
PY
}

gum_bin_path() {
  local arch="$1"
  printf '%s/%s/%s/gum' "$GUM_ROOT" "$GUM_VERSION" "$arch"
}

gum_download_and_install() {
  local arch="$1"
  local metadata tmp tarball url expected actual target_dir
  tmp="$(mktemp -d)"

  metadata="${tmp}/gum-latest.json"
  local tools_base_url="${YUNDRONE_BLE_TOOLS_BASE_URL:-https://install.yundrone.cn/yundrone/ble/tools}"
  debug "gum metadata=${tools_base_url}/gum/latest.json"
  fetch "${tools_base_url}/gum/latest.json" "$metadata"
  GUM_VERSION="$(json_value "version" "$metadata")"
  url="$(gum_json_asset_value "url" "$arch" "$metadata")"
  expected="$(gum_json_asset_value "sha256" "$arch" "$metadata")"
  tarball="${tmp}/gum.tar.gz"

  info "下载 gum TUI 引擎：$(version_text "$GUM_VERSION") / $(accent "$arch")"
  debug "gum url=${url}"
  debug "gum tarball=${tarball}"
  fetch "$url" "$tarball" progress
  actual="$(sha256_file "$tarball")"
  debug "gum sha256 expected=${expected} actual=${actual}"
  [ "$actual" = "$expected" ] || fail "gum 校验失败：${arch}"

  target_dir="${GUM_ROOT}/${GUM_VERSION}/${arch}"
  debug "gum target_dir=${target_dir}"
  rm -rf "${tmp}/extract" "$target_dir"
  mkdir -p "${tmp}/extract" "$target_dir"
  tar -xzf "$tarball" -C "${tmp}/extract"
  local found
  found="$(find "${tmp}/extract" -type f -name gum | head -n 1)"
  [ -n "$found" ] || fail "gum 安装包中没有 gum 可执行文件"
  cp "$found" "${target_dir}/gum"
  chmod +x "${target_dir}/gum"
  rm -rf "$tmp"
}

ensure_gum() {
  local arch
  arch="$(gum_arch)"
  [ "$arch" != "unsupported" ] || fail "当前架构不支持 gum TUI：$(uname -s) $(uname -m)。仅支持 Linux amd64/arm64 和 macOS arm64。"

  GUM_BIN="$(gum_bin_path "$arch")"
  if [ -x "$GUM_BIN" ]; then
    debug "gum cache hit=${GUM_BIN}"
    return 0
  fi

  gum_download_and_install "$arch"
  GUM_BIN="$(gum_bin_path "$arch")"
  [ -x "$GUM_BIN" ] || fail "gum TUI 引擎安装失败"
}

gum() {
  TERM="${TERM:-xterm-256color}" \
    COLORTERM="${COLORTERM:-truecolor}" \
    COLORFGBG="${COLORFGBG:-15;0}" \
    "$GUM_BIN" "$@"
}
