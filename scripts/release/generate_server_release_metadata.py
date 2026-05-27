#!/usr/bin/env python3
import argparse
import hashlib
import json
from datetime import datetime, timezone
from pathlib import Path


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as fh:
        for chunk in iter(lambda: fh.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def main() -> int:
    parser = argparse.ArgumentParser(description="Generate YunDrone BLE server release metadata")
    parser.add_argument("--version", required=True)
    parser.add_argument("--release-dir", type=Path, required=True)
    parser.add_argument("--base-url", default="https://install.yundrone.cn/yundrone/ble-server")
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()

    assets = {}
    for tarball in sorted(args.release_dir.glob("yundrone-ble-server-*.tar.gz")):
        platform = tarball.name.removeprefix("yundrone-ble-server-").removesuffix(".tar.gz")
        assets[platform] = {
            "file": tarball.name,
            "sha256": sha256_file(tarball),
            "url": f"{args.base_url.rstrip('/')}/releases/{args.version}/{tarball.name}",
        }

    if not assets:
        raise SystemExit(f"no server tarballs found in {args.release_dir}")

    metadata = {
        "latest": args.version,
        "version": args.version,
        "channel": "stable",
        "base_url": f"{args.base_url.rstrip('/')}/releases/{args.version}",
        "assets": assets,
        "published_at": datetime.now(timezone.utc).replace(microsecond=0).isoformat().replace("+00:00", "Z"),
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(metadata, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(f"wrote {args.output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
