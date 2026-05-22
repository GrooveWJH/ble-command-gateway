tr_text() {
  local zh="$1"
  local en="$2"
  if [ "${UI_LANG:-zh}" = "en" ]; then
    printf '%s' "$en"
  else
    printf '%s' "$zh"
  fi
}
