#!/usr/bin/env bash
set -Eeuo pipefail

LAUNCHER_BASE_URL="${YUNDRONE_BLE_LAUNCHER_BASE_URL:-https://install.yundrone.cn/yundrone/ble/launcher}"
CACHE_ROOT="${YUNDRONE_BLE_LAUNCHER_CACHE_ROOT:-${HOME}/.yundrone/ble-launcher}"
VERBOSE="no"

COLOR_RESET=""
COLOR_BOLD=""
COLOR_RED=""
COLOR_GREEN=""
COLOR_YELLOW=""
COLOR_BLUE=""
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
  COLOR_CYAN="$(tput setaf 6 || true)"
fi

strong() {
  printf '%s%s%s' "$COLOR_BOLD" "$*" "$COLOR_RESET"
}

version_text() {
  printf '%s%s%s' "$COLOR_GREEN" "$*" "$COLOR_RESET"
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
      --help|-h)
        cat <<'EOF'
YunDrone BLE 统一入口

用法:
  ble-wifi-tool.sh [命令] [选项]

命令:
  menu      打开统一 TUI，默认动作
  client    下载/启动 YunDrone BLE Client CLI
  server    打开 BLE Server 部署/管理向导
  doctor    检查 launcher、client、server 安装源

选项:
  --yes               跳过确认，适合自动化
  --verbose, -v       输出下载、缓存、校验和路径细节
  -h, --help          显示帮助

推荐:
  bash <(curl -fsSL https://install.yundrone.cn/ble-wifi-tool.sh)
EOF
        exit 0
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

verify_cached_manifest() {
  local manifest="$1"
  local base_dir="$2"
  [ -f "$manifest" ] || return 1
  have sha256sum || return 1
  have python3 || return 1

  local path expected actual
  while IFS=$'\t' read -r path expected _url; do
    [ -n "$path" ] || continue
    [ -f "${base_dir}/${path}" ] || return 1
    actual="$(sha256sum "${base_dir}/${path}" | awk '{print $1}')"
    [ "$actual" = "$expected" ] || return 1
  done < <(json_file_entries "$manifest")
}

confirm_use_cached_launcher() {
  local version="$1"
  shift
  local arg
  for arg in "$@"; do
    case "$arg" in
      --yes|-y) return 1 ;;
    esac
  done
  [ -t 0 ] || return 1

  warn "无法连接安装源，但本机已有缓存 launcher：$(version_text "$version")"
  printf '%s [y/N] ' "$(strong "是否使用本地缓存版本继续？")"
  local answer
  read -r answer || return 1
  case "$answer" in
    Y|y|yes|YES|是|好) return 0 ;;
    *) return 1 ;;
  esac
}

run_cached_or_fail() {
  local reason="$1"
  shift
  local current="${CACHE_ROOT}/current"
  local version="unknown"
  [ -f "${current}/VERSION" ] && version="$(tr -d '\r\n' <"${current}/VERSION")"
  if [ -x "${current}/main.sh" ] && confirm_use_cached_launcher "$version" "$@"; then
    exec bash "${current}/main.sh" "$@"
  fi
  fail "${reason}。请检查网络后重试；如需使用旧缓存，请交互式运行，不要加 --yes。"
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

prepare_launcher() {
  parse_bootstrap_args "$@"
  section "正在准备 YunDrone BLE 入口"

  have bash || fail "需要 bash"
  have sha256sum || run_cached_or_fail "缺少 sha256sum，无法校验 launcher" "$@"
  have python3 || run_cached_or_fail "缺少 python3，无法解析 launcher manifest" "$@"
  { have curl || have wget; } || run_cached_or_fail "缺少 curl/wget，无法下载 launcher" "$@"

  mkdir -p "${CACHE_ROOT}/versions" "${CACHE_ROOT}/tmp"
  local tmp metadata version remote_hash current current_manifest current_version current_hash staging entrypoint min_bash path expected actual url
  tmp="$(mktemp -d "${CACHE_ROOT}/tmp/bootstrap.XXXXXX")"
  metadata="${tmp}/latest.json"

  debug "launcher metadata=${LAUNCHER_BASE_URL}/installer/latest.json"
  if ! fetch "${LAUNCHER_BASE_URL}/installer/latest.json" "$metadata"; then
    rm -rf "$tmp"
    run_cached_or_fail "无法下载 launcher manifest" "$@"
  fi

  version="$(json_value "installer_version" "$metadata")"
  remote_hash="$(manifest_hash "$metadata")"
  entrypoint="$(json_value "entrypoint" "$metadata")"
  min_bash="$(json_value "min_bash_version" "$metadata")"
  ensure_min_bash_version "$min_bash"
  debug "launcher version=${version}"
  debug "launcher manifest_hash=${remote_hash}"

  current="${CACHE_ROOT}/current"
  current_manifest="${current}/MANIFEST.json"
  current_version=""
  current_hash=""
  [ -f "${current}/VERSION" ] && current_version="$(tr -d '\r\n' <"${current}/VERSION")"
  [ -f "$current_manifest" ] && current_hash="$(manifest_hash "$current_manifest" 2>/dev/null || true)"
  if [ "$current_version" = "$version" ] &&
    [ "$current_hash" = "$remote_hash" ] &&
    verify_cached_manifest "$current_manifest" "$current" &&
    [ -x "${current}/${entrypoint}" ]; then
    ok "本地 launcher 已是最新：$(version_text "$version")"
    rm -rf "$tmp"
    exec bash "${current}/${entrypoint}" "$@"
  fi

  info "发现新版 launcher，正在更新：$(version_text "$version")"
  staging="${CACHE_ROOT}/versions/.staging-${version}-$$"
  rm -rf "$staging"
  mkdir -p "$staging"

  while IFS=$'\t' read -r path expected url; do
    [ -n "$path" ] || continue
    mkdir -p "${staging}/$(dirname "$path")"
    if [ "$path" = "$entrypoint" ]; then
      fetch "$url" "${staging}/${path}" progress
    else
      fetch "$url" "${staging}/${path}"
    fi
    actual="$(sha256sum "${staging}/${path}" | awk '{print $1}')"
    debug "launcher file=${path} expected=${expected} actual=${actual}"
    [ "$actual" = "$expected" ] || fail "launcher 文件校验失败：${path}"
  done < <(json_file_entries "$metadata")

  cp "$metadata" "${staging}/MANIFEST.json"
  printf '%s\n' "$version" >"${staging}/VERSION"
  chmod +x "${staging}/${entrypoint}"
  rm -rf "${CACHE_ROOT}/versions/${version}"
  mv "$staging" "${CACHE_ROOT}/versions/${version}"
  ln -sfn "${CACHE_ROOT}/versions/${version}" "$current"
  rm -rf "$tmp"
  ok "launcher 校验通过：$(version_text "$version")"
  exec bash "${current}/${entrypoint}" "$@"
}

prepare_launcher "$@"
