import subprocess
import tempfile
import unittest
from pathlib import Path


SCRIPT = Path(__file__).with_name("extract_changelog.py")


def run_script(tmp_path: Path, version: str, changelog: str):
    changelog_path = tmp_path / "CHANGELOG"
    output_path = tmp_path / "release-notes.md"
    changelog_path.write_text(changelog, encoding="utf-8")
    result = subprocess.run(
        [
            "python3",
            str(SCRIPT),
            "--version",
            version,
            "--changelog",
            str(changelog_path),
            "--repo",
            "GrooveWJH/ble-command-gateway",
            "--output",
            str(output_path),
            "--previous-tag",
            "v2026.1.0",
            "--current-tag",
            "v2026.1.1",
        ],
        capture_output=True,
        text=True,
        check=False,
    )
    return result, output_path


class ExtractChangelogTests(unittest.TestCase):
    def test_extracts_requested_version_and_appends_compare_link(self):
        with tempfile.TemporaryDirectory() as tmp_dir:
            result, output_path = run_script(
                Path(tmp_dir),
                "2026.1.1",
                """2026.1.1 - 2026-05-18
- Added release automation.

2026.1.0 - 2026-05-17
- Initial release notes baseline.
""",
            )

            self.assertEqual(result.returncode, 0, result.stderr)
            output = output_path.read_text(encoding="utf-8")
            self.assertIn("2026.1.1 - 2026-05-18", output)
            self.assertIn("Added release automation.", output)
            self.assertIn("Platform Availability", output)
            self.assertIn("compare/v2026.1.0...v2026.1.1", output)

    def test_fails_when_version_section_is_missing(self):
        with tempfile.TemporaryDirectory() as tmp_dir:
            result, _ = run_script(
                Path(tmp_dir),
                "2026.1.1",
                """2026.1.0 - 2026-05-17
- Initial release notes baseline.
""",
            )

            self.assertEqual(result.returncode, 1)
            self.assertIn("version section not found: 2026.1.1", result.stderr)


if __name__ == "__main__":
    unittest.main()
