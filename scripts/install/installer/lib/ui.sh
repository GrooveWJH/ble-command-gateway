COLOR_RESET=""
COLOR_BOLD=""
COLOR_DIM=""
COLOR_RED=""
COLOR_GREEN=""
COLOR_YELLOW=""
COLOR_BLUE=""
COLOR_MAGENTA=""
COLOR_CYAN=""
COLOR_WHITE=""

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
  COLOR_WHITE="$(tput setaf 7 || true)"
fi

log() {
  printf '%s\n' "$*"
}

debug() {
  [ "${VERBOSE:-no}" = "yes" ] || return 0
  printf '%s%s%s %s\n' "$COLOR_DIM" "debug" "$COLOR_RESET" "$*" >&2
}

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

bad_text() {
  color_text "$COLOR_RED" "$*"
}

danger_text() {
  printf '%s%s%s%s' "$COLOR_BOLD" "$COLOR_RED" "$*" "$COLOR_RESET"
}

path_text() {
  color_text "$COLOR_CYAN" "$*"
}

command_text() {
  color_text "$COLOR_MAGENTA" "$*"
}

version_text() {
  color_text "$COLOR_GREEN" "$*"
}

menu_number() {
  printf '%s%s%s%s' "$COLOR_BOLD" "$COLOR_CYAN" "$1" "$COLOR_RESET"
}

field_line() {
  local name="$1"
  shift
  printf '  %s%s%s：%s\n' "$COLOR_DIM" "$name" "$COLOR_RESET" "$*"
}

subheading() {
  printf '\n%s%s%s\n' "$COLOR_BOLD" "$1" "$COLOR_RESET"
}

status_text() {
  case "$1" in
    OK|active|运行中|已安装，运行正常|可以安装|可以安装。*|*满足*)
      good_text "$1"
      ;;
    缺失|不支持|unsupported|不可访问|未发现|暂不可安装*|*失败*|*未运行|已安装，但服务未运行)
      bad_text "$1"
      ;;
    inactive|未运行|未安装|未创建|未知|存在半安装残留|可以安装，但有警告*|*警告*)
      warn_text "$1"
      ;;
    *)
      accent "$1"
      ;;
  esac
}

info() {
  if [ "${TUI_LOADING:-no}" = "yes" ] && tui_ready; then
    debug "suppressed info: $*"
    return 0
  fi
  printf '%s%s%s %s\n' "$COLOR_CYAN" "信息" "$COLOR_RESET" "$*"
}

ok() {
  if [ "${TUI_LOADING:-no}" = "yes" ] && tui_ready; then
    debug "suppressed ok: $*"
    return 0
  fi
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

confirm_yes() {
  local prompt="$1"
  if [ "$ASSUME_YES" = "yes" ]; then
    return 0
  fi
  if tui_ready; then
    tui_confirm "$prompt"
    return $?
  fi
  printf '%s %s ' "$(strong "$prompt")" "$(muted "[Y/n]")"
  local answer
  read -r answer || return 1
  case "$answer" in
    ""|Y|y|yes|YES|是|好)
      return 0
      ;;
    *)
      return 1
      ;;
  esac
}

confirm_no() {
  local prompt="$1"
  if [ "$ASSUME_YES" = "yes" ]; then
    return 1
  fi
  if tui_ready; then
    gum confirm \
      --affirmative "查看" \
      --negative "跳过" \
      --default=false \
      --prompt.foreground 39 \
      "$prompt"
    return $?
  fi
  printf '%s %s ' "$(strong "$prompt")" "$(muted "[y/N]")"
  local answer
  read -r answer || return 1
  case "$answer" in
    Y|y|yes|YES|是|好)
      return 0
      ;;
    *)
      return 1
      ;;
  esac
}

read_choice_into() {
  local __target="$1"
  printf '\n%s ' "$(strong "请输入序号:")"
  local __value
  if ! read -r __value; then
    printf '\n'
    exit 0
  fi
  printf -v "$__target" '%s' "$__value"
}

is_tty() {
  [ -t 0 ] && [ -t 1 ]
}

use_tui() {
  [ "${YUNDRONE_NO_TUI:-}" != "1" ] || return 1
  [ "$ASSUME_YES" != "yes" ] || return 1
  is_tty || return 1
}
