import unittest

from verify_distribution_manifest import validate_manifest


class VerifyDistributionManifestTests(unittest.TestCase):
    def test_accepts_installer_manifest(self) -> None:
        assets = validate_manifest(
            {
                "installer_version": "2026.4.0",
                "files": [
                    {
                        "url": "https://example.test/main.sh",
                        "sha256": "a" * 64,
                    }
                ],
            },
            "2026.4.0",
            "installer_version",
        )
        self.assertEqual(len(assets), 1)

    def test_accepts_release_manifest(self) -> None:
        assets = validate_manifest(
            {
                "version": "2026.4.0",
                "assets": {
                    "linux-amd64": {
                        "url": "https://example.test/client.tar.gz",
                        "sha256": "b" * 64,
                    }
                },
            },
            "2026.4.0",
            "version",
        )
        self.assertEqual(len(assets), 1)

    def test_rejects_wrong_version_or_hash(self) -> None:
        with self.assertRaisesRegex(ValueError, "mismatch"):
            validate_manifest(
                {"version": "2026.3.0", "assets": {}},
                "2026.4.0",
                "version",
            )
        with self.assertRaisesRegex(ValueError, "invalid SHA256"):
            validate_manifest(
                {
                    "version": "2026.4.0",
                    "assets": {"linux": {"url": "https://x", "sha256": "bad"}},
                },
                "2026.4.0",
                "version",
            )


if __name__ == "__main__":
    unittest.main()
