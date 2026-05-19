import unittest
from pathlib import Path
import sys

sys.path.insert(0, str(Path(__file__).resolve().parent))
import check_version_consistency


class CheckVersionConsistencyTests(unittest.TestCase):
    def test_reports_mismatched_workspace_packages(self):
        metadata = {
            "workspace_members": ["path+file:///repo/crates/server#2026.1.1"],
            "packages": [
                {
                    "id": "path+file:///repo/crates/server#2026.1.1",
                    "name": "yundrone-ble-server",
                    "version": "2026.3.22",
                },
                {
                    "id": "registry+https://github.com/rust-lang/crates.io-index#serde@1.0.0",
                    "name": "serde",
                    "version": "1.0.0",
                },
            ],
        }

        mismatches = check_version_consistency.mismatched_packages(metadata, "2026.1.1")

        self.assertEqual(
            mismatches,
            ["yundrone-ble-server: Cargo.toml=2026.3.22 VERSION=2026.1.1"],
        )

    def test_ignores_non_workspace_dependencies(self):
        metadata = {
            "workspace_members": ["path+file:///repo/crates/server#2026.1.1"],
            "packages": [
                {
                    "id": "path+file:///repo/crates/server#2026.1.1",
                    "name": "yundrone-ble-server",
                    "version": "2026.1.1",
                },
                {
                    "id": "registry+https://github.com/rust-lang/crates.io-index#serde@1.0.0",
                    "name": "serde",
                    "version": "1.0.0",
                },
            ],
        }

        self.assertEqual(
            check_version_consistency.mismatched_packages(metadata, "2026.1.1"),
            [],
        )


if __name__ == "__main__":
    unittest.main()
