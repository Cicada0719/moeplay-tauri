"""Validate an uploaded batch before atomically switching the static site.

Usage: python3 activate.py /home/user/moeplay-release 0.23.0
Upload to incoming/<version>/{site,downloads}/ via SFTP first.
"""
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import sys

root = Path(sys.argv[1]).resolve()
version = sys.argv[2]
if not re.fullmatch(r"\d+\.\d+\.\d+", version):
    raise SystemExit("Invalid version")
stage = root / "incoming" / version
current = root / "current"
if current.exists() and not current.is_symlink():
    raise SystemExit("current must be a symlink; preserve existing directory")
for name in ("index.html", "site.css", "site.js", "assets/desktop.png", "assets/reading.png"):
    if not (stage / "site" / name).is_file():
        raise SystemExit(f"Missing site file: {name}")
assets = stage / "downloads"
manifest = json.loads((assets / "release-manifest.json").read_text())
if manifest.get("schemaVersion") != 1 or manifest.get("version") != version or not re.fullmatch(r"[0-9a-f]{40}", manifest.get("commit", "")):
    raise SystemExit("Version mismatch")
if not isinstance(manifest.get("assets"), list) or not manifest["assets"]:
    raise SystemExit("Missing release assets")
latest = json.loads((assets / "latest.json").read_text())
if latest.get("version") != version or not latest.get("platforms", {}).get("windows-x86_64", {}).get("signature"):
    raise SystemExit("Invalid updater metadata")
seen = set()
for asset in manifest["assets"]:
    name = asset["file"]
    if Path(name).name != name or "/" in name or "\\" in name:
        raise SystemExit("Unsafe asset filename")
    if name in seen:
        raise SystemExit("Duplicate asset filename")
    seen.add(name)
    file = assets / name
    with file.open("rb") as stream:
        digest = hashlib.file_digest(stream, "sha256").hexdigest()
    if file.stat().st_size != asset["size"] or digest != asset["sha256"]:
        raise SystemExit(f"Hash mismatch: {name}")
site = root / "sites" / version
download = root / "downloads" / version
if site.exists() or download.exists():
    raise SystemExit("Version already exists; old files will not be replaced")
site.parent.mkdir(parents=True, exist_ok=True)
download.parent.mkdir(parents=True, exist_ok=True)
shutil.copytree(stage / "site", site)
for name in ("release-manifest.json", "latest.json"):
    shutil.copy2(assets / name, site / name)
versions = sorted([p.name for p in site.parent.iterdir() if p.is_dir() and re.fullmatch(r"\d+\.\d+\.\d+", p.name)], key=lambda s: tuple(map(int, s.split("."))))
(site / "versions.json").write_text(json.dumps(versions))
shutil.move(str(assets), download)
previous = os.readlink(current) if current.is_symlink() else None
next_link = root / f"current-{version}"
next_link.symlink_to(Path("sites") / version, target_is_directory=True)
os.replace(next_link, current)
with (root / "deployment-history.jsonl").open("a") as log:
    log.write(json.dumps({"version": version, "commit": manifest["commit"], "previous": previous, "current": str(site)}) + "\n")
print(f"Activated {version}; previous={previous}")
