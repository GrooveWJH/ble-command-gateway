have() {
  command -v "$1" >/dev/null 2>&1
}

is_tty() {
  [ -t 0 ] && [ -t 1 ]
}

use_tui() {
  [ "${YUNDRONE_NO_TUI:-}" != "1" ] || return 1
  [ "${ASSUME_YES:-no}" != "yes" ] || return 1
  is_tty || return 1
}

is_linux() {
  [ "$(uname -s)" = "Linux" ]
}

detect_platform() {
  local os machine
  os="$(uname -s)"
  machine="$(uname -m)"
  case "${os}:${machine}" in
    Darwin:arm64|Darwin:aarch64)
      printf '%s' "macos-arm64"
      ;;
    Linux:x86_64|Linux:amd64)
      printf '%s' "linux-amd64"
      ;;
    Linux:aarch64|Linux:arm64)
      printf '%s' "linux-arm64"
      ;;
    *)
      printf '%s' "unsupported"
      ;;
  esac
}

tool_ok() {
  have "$1" && printf '%s' "OK" || printf '%s' "缺失"
}

have_sha256() {
  have sha256sum || have shasum
}

sha256_file() {
  local file="$1"
  if have sha256sum; then
    sha256sum "$file" | awk '{print $1}'
  elif have shasum; then
    shasum -a 256 "$file" | awk '{print $1}'
  else
    fail "需要 sha256sum 或 shasum"
  fi
}

url_ok() {
  local url="$1"
  if have curl; then
    curl -fsIL "$url" >/dev/null 2>&1
  elif have wget; then
    wget --spider -q "$url" >/dev/null 2>&1
  else
    return 1
  fi
}
