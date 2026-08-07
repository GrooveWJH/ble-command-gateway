#!/usr/bin/env python3
import argparse
import hashlib
import json
import re
import urllib.request
from collections.abc import Iterable
from typing import Any


SHA256_RE = re.compile(r"^[0-9a-f]{64}$")


def release_assets(manifest: dict[str, Any]) -> list[dict[str, str]]:
    if isinstance(manifest.get("files"), list):
        return manifest["files"]
    assets = manifest.get("assets")
    if isinstance(assets, dict):
        return list(assets.values())
    raise ValueError("manifest contains no files or assets")


def validate_manifest(
    manifest: dict[str, Any], expected_version: str, version_field: str
) -> list[dict[str, str]]:
    actual = manifest.get(version_field)
    if actual != expected_version:
        raise ValueError(
            f"{version_field} mismatch: expected {expected_version}, got {actual}"
        )
    assets = release_assets(manifest)
    if not assets:
        raise ValueError("manifest asset list is empty")
    for asset in assets:
        if not isinstance(asset.get("url"), str):
            raise ValueError("asset URL is missing")
        sha256 = asset.get("sha256")
        if not isinstance(sha256, str) or not SHA256_RE.fullmatch(sha256):
            raise ValueError(f"invalid SHA256 for {asset.get('url')}")
    return assets


def response_chunks(response: Any) -> Iterable[bytes]:
    while chunk := response.read(1024 * 1024):
        yield chunk


def download_json(url: str) -> dict[str, Any]:
    with urllib.request.urlopen(url, timeout=30) as response:
        return json.load(response)


def verify_asset(asset: dict[str, str]) -> None:
    digest = hashlib.sha256()
    with urllib.request.urlopen(asset["url"], timeout=60) as response:
        for chunk in response_chunks(response):
            digest.update(chunk)
    actual = digest.hexdigest()
    if actual != asset["sha256"]:
        raise ValueError(
            f"asset SHA256 mismatch for {asset['url']}: "
            f"expected {asset['sha256']}, got {actual}"
        )
    print(f"verified {asset['url']}")


def main() -> int:
    parser = argparse.ArgumentParser(description="Verify a live YunDrone manifest")
    parser.add_argument("--manifest-url", required=True)
    parser.add_argument("--expected-version", required=True)
    parser.add_argument("--version-field", default="version")
    args = parser.parse_args()

    manifest = download_json(args.manifest_url)
    assets = validate_manifest(manifest, args.expected_version, args.version_field)
    for asset in assets:
        verify_asset(asset)
    print(f"verified {args.manifest_url} at {args.expected_version}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
