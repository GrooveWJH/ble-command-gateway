COLOR_RESET=""
COLOR_BOLD=""
COLOR_DIM=""
COLOR_RED=""
COLOR_GREEN=""
COLOR_YELLOW=""
COLOR_BLUE=""
COLOR_MAGENTA=""
COLOR_CYAN=""

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

bad_text() {
  color_text "$COLOR_RED" "$*"
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
  [ "${VERBOSE:-no}" = "yes" ] || return 0
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

field_line() {
  local name="$1"
  shift
  printf '  %s%s%s：%s\n' "$COLOR_DIM" "$name" "$COLOR_RESET" "$*"
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
