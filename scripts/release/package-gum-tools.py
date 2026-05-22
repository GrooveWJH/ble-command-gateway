#!/usr/bin/env python3
import argparse
import hashlib
import json
import tarfile
import tempfile
import urllib.request
from datetime import datetime, timezone
from pathlib import Path


ASSETS = {
    "linux-amd64": "gum_{version}_Linux_x86_64.tar.gz",
    "linux-arm64": "gum_{version}_Linux_arm64.tar.gz",
    "macos-arm64": "gum_{version}_Darwin_arm64.tar.gz",
}


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as fh:
        for chunk in iter(lambda: fh.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def download(url: str, path: Path) -> None:
    print(f"download {url}")
    with urllib.request.urlopen(url) as response, path.open("wb") as fh:
        fh.write(response.read())


def find_gum(extract_dir: Path) -> Path:
    matches = list(extract_dir.rglob("gum"))
    if not matches:
        raise RuntimeError("gum binary not found in upstream asset")
    return matches[0]


def main() -> int:
    parser = argparse.ArgumentParser(description="Package gum tools for YunDrone installer")
    parser.add_argument("--version", default="0.17.0")
    parser.add_argument("--output-dir", type=Path, required=True)
    parser.add_argument("--base-url", default="https://install.yundrone.cn/yundrone/ble/tools/gum")
    parser.add_argument("--platform", action="append", choices=sorted(ASSETS), dest="platforms")
    args = parser.parse_args()

    platforms = args.platforms or sorted(ASSETS)
    version_dir = args.output_dir / "versions" / args.version
    version_dir.mkdir(parents=True, exist_ok=True)
    assets = {}

    with tempfile.TemporaryDirectory() as tmp_name:
        tmp = Path(tmp_name)
        for platform in platforms:
            upstream_file = ASSETS[platform].format(version=args.version)
            upstream_url = f"https://github.com/charmbracelet/gum/releases/download/v{args.version}/{upstream_file}"
            upstream_tarball = tmp / upstream_file
            download(upstream_url, upstream_tarball)
            upstream_sha = sha256_file(upstream_tarball)

            extract_dir = tmp / f"extract-{platform}"
            extract_dir.mkdir()
            with tarfile.open(upstream_tarball, "r:gz") as archive:
                archive.extractall(extract_dir)
            gum = find_gum(extract_dir)

            package_name = f"gum-{platform}.tar.gz"
            package_path = version_dir / package_name
            with tarfile.open(package_path, "w:gz", format=tarfile.USTAR_FORMAT) as archive:
                archive.add(gum, arcname="gum")
            package_sha = sha256_file(package_path)

            assets[platform] = {
                "file": package_name,
                "sha256": package_sha,
                "upstream_file": upstream_file,
                "upstream_sha256": upstream_sha,
                "url": f"{args.base_url.rstrip('/')}/versions/{args.version}/{package_name}",
            }

    latest = {
        "assets": assets,
        "published_at": datetime.now(timezone.utc).replace(microsecond=0).isoformat().replace("+00:00", "Z"),
        "tool": "gum",
        "version": args.version,
    }
    (args.output_dir / "latest.json").write_text(json.dumps(latest, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(f"wrote {args.output_dir / 'latest.json'}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
