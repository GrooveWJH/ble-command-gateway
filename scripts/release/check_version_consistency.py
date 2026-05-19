#!/usr/bin/env python3
import json
import subprocess
import sys
from pathlib import Path


def read_version(root: Path) -> str:
    return (root / "VERSION").read_text(encoding="utf-8").strip()


def cargo_metadata(root: Path) -> dict:
    result = subprocess.run(
        ["cargo", "metadata", "--format-version", "1", "--no-deps"],
        cwd=root,
        capture_output=True,
        text=True,
        check=False,
    )
    if result.returncode != 0:
        raise RuntimeError(result.stderr.strip() or result.stdout.strip())
    return json.loads(result.stdout)


def mismatched_packages(metadata: dict, expected_version: str) -> list[str]:
    workspace_members = set(metadata["workspace_members"])
    mismatches = []
    for package in metadata["packages"]:
        if package["id"] not in workspace_members:
            continue
        if package["version"] != expected_version:
            mismatches.append(
                f"{package['name']}: Cargo.toml={package['version']} VERSION={expected_version}"
            )
    return mismatches


def main() -> int:
    root = Path(__file__).resolve().parents[2]
    expected_version = read_version(root)
    metadata = cargo_metadata(root)
    mismatches = mismatched_packages(metadata, expected_version)
    if mismatches:
        print("workspace package versions do not match VERSION:", file=sys.stderr)
        for mismatch in mismatches:
            print(f"- {mismatch}", file=sys.stderr)
        return 1
    print(f"VERSION matches all workspace packages: {expected_version}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
