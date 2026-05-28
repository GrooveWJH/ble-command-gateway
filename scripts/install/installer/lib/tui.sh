tui_ready() {
  use_tui && [ -n "${GUM_BIN:-}" ] && [ -x "$GUM_BIN" ]
}

tui_style() {
  gum style "$@"
}

tui_clear() {
  printf '\033[2J\033[H'
}

tui_pause() {
  gum input --placeholder "按回车返回" >/dev/null || true
}

tui_pager() {
  gum pager "$@"
}

tui_title() {
  gum style \
    --foreground 15 \
    --background 24 \
    --bold \
    --width 72 \
    --padding "0 1" \
    --margin "1 0 0 2" \
    "YunDrone BLE Server"
  gum style \
    --foreground 245 \
    --width 72 \
    --margin "0 0 1 2" \
    "BLE provisioning and diagnostics for headless edge devices"
}

tui_card() {
  gum style \
    --border rounded \
    --border-foreground "${2:-39}" \
    --width "${TUI_WIDTH:-64}" \
    --padding "1 2" \
    --margin "0 0 1 2" \
    "$1"
}

tui_info_card() {
  tui_card "$1" 39
}

tui_warn_card() {
  tui_card "$1" 214
}

tui_danger_card() {
  tui_card "$1" 196
}

tui_success_card() {
  tui_card "$1" 42
}

tui_choose() {
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
    --affirmative "继续" \
    --negative "取消" \
    --prompt.foreground 214 \
    "$1"
}

tui_confirm_danger() {
  tui_danger_card "$1"
  gum confirm \
    --affirmative "确认执行" \
    --negative "取消" \
    --prompt.foreground 196 \
    "这是危险操作，是否继续？"
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

tui_step() {
  gum style --foreground 39 "• $1"
}

tui_run_step() {
  local title="$1"
  shift
  if ! tui_ready; then
    "$@"
    return $?
  fi

  local log_file status
  log_file="$(mktemp)"
  (
    export TUI_LOADING=yes
    "$@"
  ) >"$log_file" 2>&1 &
  local pid=$!
  gum spin \
    --spinner dot \
    --spinner.foreground 39 \
    --title.foreground 212 \
    --title "$title" \
    -- bash -c 'while kill -0 "$1" 2>/dev/null; do sleep 0.2; done' _ "$pid"

  wait "$pid"
  status=$?

  if [ "$status" -ne 0 ]; then
    tui_danger_card "步骤失败：${title}"
    cat "$log_file" >&2
    rm -f "$log_file"
    return "$status"
  fi

  rm -f "$log_file"
  gum style --foreground 42 "✓ ${title}"
}

tui_table() {
  gum table \
    --border rounded \
    --separator "  " \
    --columns "$@"
}
