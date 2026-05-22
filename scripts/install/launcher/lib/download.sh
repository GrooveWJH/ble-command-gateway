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
    fail "需要 curl 或 wget"
  fi
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

json_asset_value() {
  local key="$1"
  local platform="$2"
  local file="$3"
  python3 - "$key" "$platform" "$file" <<'PY'
import json
import sys

key, platform, path = sys.argv[1:]
with open(path, "r", encoding="utf-8") as fh:
    data = json.load(fh)

value = data["assets"][platform]
for part in key.split("."):
    value = value[part]
print(value)
PY
}
