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


def manifest_hash(files: list[dict[str, str]]) -> str:
    digest = hashlib.sha256()
    for entry in sorted(files, key=lambda item: item["path"]):
        digest.update(entry["path"].encode("utf-8"))
        digest.update(b"\0")
        digest.update(entry["sha256"].encode("utf-8"))
        digest.update(b"\n")
    return digest.hexdigest()


def collect_files(source: Path, base_url: str, version: str) -> list[dict[str, str]]:
    files = []
    for path in sorted(source.rglob("*")):
        if not path.is_file():
            continue
        relative = path.relative_to(source).as_posix()
        files.append(
            {
                "path": relative,
                "sha256": sha256_file(path),
                "url": f"{base_url.rstrip('/')}/installer/versions/{version}/{relative}",
            }
        )
    return files


def main() -> int:
    parser = argparse.ArgumentParser(description="Generate YunDrone shell installer manifest")
    parser.add_argument("--source", type=Path, required=True)
    parser.add_argument("--version", required=True)
    parser.add_argument("--base-url", required=True)
    parser.add_argument("--entrypoint", default="main.sh")
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--min-bash-version", default="3.2")
    args = parser.parse_args()

    files = collect_files(args.source, args.base_url, args.version)
    manifest = {
        "entrypoint": args.entrypoint,
        "files": files,
        "installer_version": args.version,
        "manifest_hash": manifest_hash(files),
        "min_bash_version": args.min_bash_version,
        "published_at": datetime.now(timezone.utc).replace(microsecond=0).isoformat().replace("+00:00", "Z"),
    }

    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(manifest, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(f"wrote {args.output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
