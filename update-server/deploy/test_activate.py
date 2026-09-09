import hashlib
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

SCRIPT = Path(__file__).with_name("activate.py")


class DeploymentTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="moeplay-deploy-test-")
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.stage = self.root / "incoming" / "0.23.0"
        site = self.stage / "site"
        (site / "assets").mkdir(parents=True)
        for name in ("index.html", "site.css", "site.js", "assets/desktop.png", "assets/reading.png"):
            (site / name).write_bytes(b"fixture")
        self.assets = self.stage / "downloads"
        self.assets.mkdir()
        (self.assets / "MoeGame_0.23.0.exe").write_bytes(b"installer")
        self.manifest = {"schemaVersion": 1, "version": "0.23.0", "commit": "a" * 40,
                         "assets": [{"file": "MoeGame_0.23.0.exe", "size": 9,
                                     "sha256": hashlib.sha256(b"installer").hexdigest()}]}
        self.write_manifest()
        (self.assets / "latest.json").write_text(json.dumps({"version": "0.23.0", "platforms": {"windows-x86_64": {"signature": "fixture"}}}))

    def write_manifest(self):
        (self.assets / "release-manifest.json").write_text(json.dumps(self.manifest))

    def run_deploy(self):
        return subprocess.run([sys.executable, str(SCRIPT), str(self.root), "0.23.0"], capture_output=True, text=True)

    def test_bad_hash_never_switches_or_moves_assets(self):
        (self.assets / "MoeGame_0.23.0.exe").write_bytes(b"tampered")
        result = self.run_deploy()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("Hash mismatch", result.stderr)
        self.assertFalse((self.root / "current").exists())
        self.assertTrue(self.assets.exists())

    def test_unsafe_filename_rejected(self):
        self.manifest["assets"][0]["file"] = "../outside"
        self.write_manifest()
        self.assertIn("Unsafe asset", self.run_deploy().stderr)

    def test_existing_version_preserved(self):
        existing = self.root / "sites" / "0.23.0"
        existing.mkdir(parents=True)
        (existing / "sentinel").write_text("old")
        self.assertIn("already exists", self.run_deploy().stderr)
        self.assertEqual((existing / "sentinel").read_text(), "old")

    def test_success_uses_relative_link_and_keeps_old_download(self):
        probe = self.root / "symlink-probe"
        try:
            probe.symlink_to("incoming", target_is_directory=True)
            probe.unlink()
        except OSError:
            self.skipTest("Host does not allow symlinks; run on the Linux deployment host")
        old = self.root / "downloads" / "old.exe"
        old.parent.mkdir()
        old.write_bytes(b"old")
        result = self.run_deploy()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual((self.root / "current").readlink().as_posix(), "sites/0.23.0")
        self.assertTrue((self.root / "current" / "index.html").is_file())
        self.assertEqual(old.read_bytes(), b"old")
        self.assertEqual(json.loads((self.root / "current" / "versions.json").read_text()), ["0.23.0"])


if __name__ == "__main__":
    unittest.main()
