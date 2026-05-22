resolve_release_asset() {
  local arch="$1"
  local tmp="$2"
  local metadata="${tmp}/latest.json"
  local selected_version asset_url expected_sha

  if [ "$VERSION" = "latest" ]; then
    debug "release metadata=${INSTALL_BASE_URL}/latest.json"
    fetch "${INSTALL_BASE_URL}/latest.json" "$metadata"
    selected_version="$(json_value "latest" "$metadata")"
  else
    selected_version="$VERSION"
    debug "release metadata=${INSTALL_BASE_URL}/releases/${selected_version}/release.json"
    fetch "${INSTALL_BASE_URL}/releases/${selected_version}/release.json" "$metadata"
  fi
  debug "release selected_version=${selected_version}"

  asset_url="$(json_value "assets.${arch}.url" "$metadata" 2>/dev/null || true)"
  expected_sha="$(json_value "assets.${arch}.sha256" "$metadata" 2>/dev/null || true)"
  [ -n "$asset_url" ] && [ -n "$expected_sha" ] || fail "安装源没有提供 ${arch} 的 server 包"

  local tarball="${tmp}/server.tar.gz"
  printf '%s\n' "$asset_url" >"${tmp}/asset-url"
  printf '%s\n' "$expected_sha" >"${tmp}/expected-sha"
  printf '%s\n' "$selected_version" >"${tmp}/selected-version"
  printf '%s\n' "$tarball" >"${tmp}/tarball-path"
}

download_release_tarball() {
  local tmp="$1"
  local asset_url tarball
  asset_url="$(cat "${tmp}/asset-url")"
  tarball="$(cat "${tmp}/tarball-path")"
  info "下载 ${asset_url}"
  debug "release tarball=${tarball}"
  fetch "$asset_url" "$tarball" progress
}

verify_release_tarball() {
  local arch="$1"
  local tmp="$2"
  local tarball expected_sha actual_sha
  tarball="$(cat "${tmp}/tarball-path")"
  expected_sha="$(cat "${tmp}/expected-sha")"
  info "校验 sha256"
  actual_sha="$(sha256_file "$tarball")"
  debug "release sha256 expected=${expected_sha} actual=${actual_sha}"
  [ "$actual_sha" = "$expected_sha" ] || fail "server release 校验失败：${arch}"
  ok "下载包校验通过"
}

download_release() {
  local arch="$1"
  local tmp="$2"
  resolve_release_asset "$arch" "$tmp"
  download_release_tarball "$tmp"
  verify_release_tarball "$arch" "$tmp"
}

extract_release() {
  local tarball="$1"
  local staging_dir="$2"
  info "解包 server 文件"
  debug "extract tarball=${tarball}"
  debug "extract staging_dir=${staging_dir}"
  if tar --help 2>/dev/null | grep -q -- '--warning'; then
    run_root tar --warning=no-unknown-keyword -xzf "$tarball" -C "$staging_dir"
  else
    run_root tar -xzf "$tarball" -C "$staging_dir" 2> >(grep -v 'LIBARCHIVE.xattr' >&2)
  fi
  run_root find "$staging_dir" -type d -exec chmod 755 {} +
  run_root find "$staging_dir" -type f -name '._*' -delete
  run_root find "$staging_dir" -type f -exec chmod 644 {} +
}
