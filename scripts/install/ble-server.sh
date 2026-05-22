#!/usr/bin/env bash
set -Eeuo pipefail

INSTALL_BASE_URL="${YUNDRONE_INSTALL_BASE_URL:-https://install.yundrone.cn/yundrone/ble-server}"
CACHE_ROOT="${YUNDRONE_INSTALLER_CACHE_ROOT:-${HOME}/.yundrone/ble-server-installer}"
BOOTSTRAP_SOURCE_NAME="${YUNDRONE_BOOTSTRAP_SOURCE_NAME:-ble-server.sh}"
VERBOSE="no"

COLOR_RESET=""
COLOR_BOLD=""
COLOR_RED=""
COLOR_GREEN=""
COLOR_YELLOW=""
COLOR_BLUE=""
COLOR_MAGENTA=""
COLOR_CYAN=""
COLOR_DIM=""

if [ -t 1 ] && [ -z "${NO_COLOR:-}" ] && [ "${TERM:-}" != "dumb" ] && command -v tput >/dev/null 2>&1; then
  COLOR_RESET="$(tput sgr0 || true)"
  COLOR_BOLD="$(tput bold || true)"
  COLOR_DIM="$(tput dim || true)"
  COLOR_RED="$(tput setaf 1 || true)"
  COLOR_GREEN="$(tput setaf 2 || true)"
  COLOR_YELLOW="$(tput setaf 3 || true)"
  COLOR_BLUE="$(tput setaf 4 || true)"
  COLOR_MAGENTA="$(tput setaf 5 || true)"
  COLOR_CYAN="$(tput setaf 6 || true)"
fi

color_text() {
  local color="$1"
  shift
  printf '%s%s%s' "$color" "$*" "$COLOR_RESET"
}

strong() {
  color_text "$COLOR_BOLD" "$*"
}

muted() {
  color_text "$COLOR_DIM" "$*"
}

accent() {
  color_text "$COLOR_CYAN" "$*"
}

good_text() {
  color_text "$COLOR_GREEN" "$*"
}

warn_text() {
  color_text "$COLOR_YELLOW" "$*"
}

path_text() {
  color_text "$COLOR_CYAN" "$*"
}

version_text() {
  color_text "$COLOR_GREEN" "$*"
}

info() {
  printf '%s%s%s %s\n' "$COLOR_CYAN" "信息" "$COLOR_RESET" "$*"
}

debug() {
  [ "$VERBOSE" = "yes" ] || return 0
  printf '%s%s%s %s\n' "$COLOR_DIM" "debug" "$COLOR_RESET" "$*" >&2
}

ok() {
  printf '%s%s%s %s\n' "$COLOR_GREEN" "完成" "$COLOR_RESET" "$*"
}

warn() {
  printf '%s%s%s %s\n' "$COLOR_YELLOW" "注意" "$COLOR_RESET" "$*" >&2
}

fail() {
  printf '%s%s%s %s\n' "$COLOR_RED" "错误" "$COLOR_RESET" "$*" >&2
  exit 1
}

section() {
  printf '\n%s%s%s\n\n' "$COLOR_BOLD$COLOR_BLUE" "$1" "$COLOR_RESET"
}

have() {
  command -v "$1" >/dev/null 2>&1
}

fetch() {
  local url="$1"
  local output="$2"
  local mode="${3:-quiet}"
  debug "fetch url=${url}"
  debug "fetch output=${output}"
  if have curl; then
    if [ "$mode" = "progress" ] && [ -t 1 ]; then
      curl -fL --progress-bar "$url" -o "$output"
    else
      curl -fsSL "$url" -o "$output"
    fi
  elif have wget; then
    if [ "$mode" = "progress" ] && [ -t 1 ]; then
      wget --show-progress -O "$output" "$url"
    else
      wget -qO "$output" "$url"
    fi
  else
    return 1
  fi
}

parse_bootstrap_args() {
  local arg
  for arg in "$@"; do
    case "$arg" in
      --verbose|-v)
        VERBOSE="yes"
        ;;
    esac
  done
}

json_value() {
  local key="$1"
  local file="$2"
  python3 - "$key" "$file" <<'PY'
import json
import sys

key = sys.argv[1]
path = sys.argv[2]
with open(path, "r", encoding="utf-8") as fh:
    data = json.load(fh)

value = data
for part in key.split("."):
    value = value[part]
print(value)
PY
}

json_file_entries() {
  local file="$1"
  python3 - "$file" <<'PY'
import json
import sys

with open(sys.argv[1], "r", encoding="utf-8") as fh:
    data = json.load(fh)

for entry in data["files"]:
    print(f"{entry['path']}\t{entry['sha256']}\t{entry['url']}")
PY
}

manifest_hash() {
  local file="$1"
  python3 - "$file" <<'PY'
import hashlib
import json
import sys

with open(sys.argv[1], "r", encoding="utf-8") as fh:
    data = json.load(fh)

value = data.get("manifest_hash")
if value:
    print(value)
    raise SystemExit(0)

digest = hashlib.sha256()
for entry in sorted(data["files"], key=lambda item: item["path"]):
    digest.update(entry["path"].encode("utf-8"))
    digest.update(b"\0")
    digest.update(entry["sha256"].encode("utf-8"))
    digest.update(b"\n")
print(digest.hexdigest())
PY
}

ensure_min_bash_version() {
  local required="$1"
  local required_major required_minor
  required_major="${required%%.*}"
  required_minor="${required#*.}"
  required_minor="${required_minor%%.*}"
  if [ "${BASH_VERSINFO[0]}" -lt "$required_major" ] ||
    { [ "${BASH_VERSINFO[0]}" -eq "$required_major" ] && [ "${BASH_VERSINFO[1]}" -lt "$required_minor" ]; }; then
    fail "当前 Bash 版本过低：${BASH_VERSION}，需要 ${required} 或更高版本"
  fi
}

confirm_use_cached_installer() {
  local version="$1"
  shift
  local arg
  for arg in "$@"; do
    case "$arg" in
      --yes|-y) return 1 ;;
    esac
  done
  if [ ! -t 0 ]; then
    return 1
  fi

  warn "无法连接安装源，但本机已有缓存安装器：$(version_text "$version")"
  printf '%s %s ' "$(strong "是否使用本地缓存版本继续？")" "$(muted "[y/N]")"
  local answer
  read -r answer || return 1
  case "$answer" in
    Y|y|yes|YES|是|好) return 0 ;;
    *) return 1 ;;
  esac
}

verify_cached_manifest() {
  local manifest="$1"
  local base_dir="$2"
  [ -f "$manifest" ] || return 1
  { have sha256sum || have shasum; } || return 1
  have python3 || return 1

  local path expected actual
  while IFS=$'\t' read -r path expected _url; do
    [ -n "$path" ] || continue
    [ -f "${base_dir}/${path}" ] || return 1
    if have sha256sum; then
      actual="$(sha256sum "${base_dir}/${path}" | awk '{print $1}')"
    else
      actual="$(shasum -a 256 "${base_dir}/${path}" | awk '{print $1}')"
    fi
    [ "$actual" = "$expected" ] || return 1
  done < <(json_file_entries "$manifest")
}

run_cached_or_fail() {
  local reason="$1"
  shift
  local current="${CACHE_ROOT}/current"
  local version="unknown"
  if [ -f "${current}/VERSION" ]; then
    version="$(tr -d '\r\n' <"${current}/VERSION")"
  fi
  if [ -x "${current}/main.sh" ] && confirm_use_cached_installer "$version" "$@"; then
    exec bash "${current}/main.sh" "$@"
  fi
  fail "${reason}。请检查网络后重试；如需使用旧缓存，请交互式运行，不要加 --yes。"
}

prepare_installer() {
  parse_bootstrap_args "$@"
  section "正在准备 YunDrone BLE Server 安装器"

  have bash || fail "需要 bash"
  { have sha256sum || have shasum; } || run_cached_or_fail "缺少 sha256sum/shasum，无法校验安装器" "$@"
  have python3 || run_cached_or_fail "缺少 python3，无法解析安装器 manifest" "$@"
  { have curl || have wget; } || run_cached_or_fail "缺少 curl/wget，无法下载安装器" "$@"

  local tmp_dir metadata installer_version installer_hash current current_manifest current_version current_hash
  mkdir -p "${CACHE_ROOT}/versions" "${CACHE_ROOT}/downloads" "${CACHE_ROOT}/tmp"
  tmp_dir="$(mktemp -d "${CACHE_ROOT}/tmp/bootstrap.XXXXXX")"
  trap 'rm -rf "$tmp_dir"' EXIT

  metadata="${tmp_dir}/latest.json"
  debug "installer metadata=${INSTALL_BASE_URL}/installer/latest.json"
  if ! fetch "${INSTALL_BASE_URL}/installer/latest.json" "$metadata"; then
    run_cached_or_fail "无法下载安装器 manifest" "$@"
  fi

  installer_version="$(json_value "installer_version" "$metadata")"
  installer_hash="$(manifest_hash "$metadata")"
  debug "installer version=${installer_version}"
  debug "installer manifest_hash=${installer_hash}"
  ensure_min_bash_version "$(json_value "min_bash_version" "$metadata")"
  current="${CACHE_ROOT}/current"
  current_manifest="${current}/MANIFEST.json"
  current_version=""
  if [ -f "${current}/VERSION" ]; then
    current_version="$(tr -d '\r\n' <"${current}/VERSION")"
  fi
  current_hash=""
  if [ -f "$current_manifest" ]; then
    current_hash="$(manifest_hash "$current_manifest" 2>/dev/null || true)"
  fi
  debug "current version=${current_version:-none}"
  debug "current manifest_hash=${current_hash:-none}"

  if [ "$current_version" = "$installer_version" ] &&
    [ "$current_hash" = "$installer_hash" ] &&
    verify_cached_manifest "$current_manifest" "$current"; then
    ok "本地安装器已是最新：$(version_text "$installer_version")"
    rm -rf "$tmp_dir"
    trap - EXIT
    exec bash "${current}/main.sh" "$@"
  fi

  if [ "$current_version" = "$installer_version" ]; then
    info "发现安装器内容更新，正在刷新：$(version_text "$installer_version")"
  else
    info "发现新版安装器，正在更新：$(version_text "$installer_version")"
  fi
  local staging="${tmp_dir}/staging"
  mkdir -p "$staging"
  cp "$metadata" "${staging}/MANIFEST.json"

  local path expected url target actual
  while IFS=$'\t' read -r path expected url; do
    [ -n "$path" ] || continue
    target="${staging}/${path}"
    mkdir -p "$(dirname "$target")"
    info "下载 $(path_text "$path")"
    if ! fetch "$url" "$target"; then
      run_cached_or_fail "下载 ${path} 失败" "$@"
    fi
    if have sha256sum; then
      actual="$(sha256sum "$target" | awk '{print $1}')"
    else
      actual="$(shasum -a 256 "$target" | awk '{print $1}')"
    fi
    debug "sha256 ${path} expected=${expected} actual=${actual}"
    if [ "$actual" != "$expected" ]; then
      run_cached_or_fail "安装器文件校验失败：${path}" "$@"
    fi
  done < <(json_file_entries "$metadata")

  printf '%s\n' "$installer_version" >"${staging}/VERSION"
  chmod +x "${staging}/main.sh"
  local version_dir="${CACHE_ROOT}/versions/${installer_version}"
  rm -rf "$version_dir"
  mkdir -p "$(dirname "$version_dir")"
  mv "$staging" "$version_dir"
  ln -sfn "$version_dir" "${CACHE_ROOT}/current"
  ok "安装器校验通过：$(version_text "$installer_version")"
  rm -rf "$tmp_dir"
  trap - EXIT
  exec bash "${CACHE_ROOT}/current/main.sh" "$@"
}

prepare_installer "$@"
