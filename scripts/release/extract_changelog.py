#!/usr/bin/env python3
import argparse
import re
import sys
from pathlib import Path


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--version", required=True)
    parser.add_argument("--changelog", default="CHANGELOG")
    parser.add_argument("--repo", required=True)
    parser.add_argument("--output", required=True)
    parser.add_argument("--previous-tag")
    parser.add_argument("--current-tag")
    return parser.parse_args()


def extract_section(content: str, version: str) -> str:
    pattern = re.compile(
        rf"^{re.escape(version)}\s+-\s+.+?$" r"(.*?)(?=^\d+\.\d+\.\d+\s+-\s+|\Z)",
        re.MULTILINE | re.DOTALL,
    )
    match = pattern.search(content)
    if not match:
        raise ValueError(f"version section not found: {version}")
    section = match.group(0).strip()
    return section


def build_compare_line(repo: str, previous_tag: str | None, current_tag: str | None) -> str:
    if not previous_tag or not current_tag:
        return ""
    return (
        f"**Full changelog**: "
        f"https://github.com/{repo}/compare/{previous_tag}...{current_tag}"
    )


def main() -> int:
    args = parse_args()
    changelog_path = Path(args.changelog)
    output_path = Path(args.output)

    content = changelog_path.read_text(encoding="utf-8")
    try:
        section = extract_section(content, args.version)
    except ValueError as exc:
        print(str(exc), file=sys.stderr)
        return 1

    release_notes = [section, "", "## Platform Availability", ""]
    release_notes.extend(
        [
            "- macOS: official prebuilt `.app` package is attached for Apple Silicon.",
            "- Linux: source deployment and systemd documentation are supported; no official prebuilt package is attached yet.",
            "- Windows: CI verifies desktop builds, but no official prebuilt package is attached yet.",
        ]
    )

    compare_line = build_compare_line(args.repo, args.previous_tag, args.current_tag)
    if compare_line:
        release_notes.extend(["", compare_line])

    output_path.write_text("\n".join(release_notes).strip() + "\n", encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
