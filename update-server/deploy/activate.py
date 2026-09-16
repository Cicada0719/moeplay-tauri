"""Validate an uploaded batch before atomically switching the static site.

Usage: python3 activate.py /home/user/moeplay-release 0.23.0 [--public-key path]
Upload to incoming/<version>/{site,downloads}/ via SFTP first.
"""
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import sys
from urllib.parse import urlparse


# Small dependency-free Ed25519 verifier. The deployment host is intentionally
# kept free of Python packages; this verifies the same Minisign packets used by
# the updater client before current is switched.
Q = 2 ** 255 - 19
L = 2 ** 252 + 27742317777372353535851937790883648493
D = (-121665 * pow(121666, Q - 2, Q)) % Q
I = pow(2, (Q - 1) // 4, Q)
_by = 4 * pow(5, Q - 2, Q) % Q
_bx = pow((_by * _by - 1) * pow((D * _by * _by + 1) % Q, Q - 2, Q) % Q, (Q + 3) // 8, Q)
if (_bx * _bx - (_by * _by - 1) * pow((D * _by * _by + 1) % Q, Q - 2, Q)) % Q:
    _bx = _bx * I % Q
B = (_bx if _bx % 2 == 0 else Q - _bx, _by)


def _inv(value):
    return pow(value, Q - 2, Q)


def _add(left, right):
    x1, y1 = left
    x2, y2 = right
    denominator_x = _inv((1 + D * x1 * x2 * y1 * y2) % Q)
    denominator_y = _inv((1 - D * x1 * x2 * y1 * y2) % Q)
    return ((x1 * y2 + x2 * y1) * denominator_x % Q,
            (y1 * y2 + x1 * x2) * denominator_y % Q)


def _mul(point, scalar):
    result = (0, 1)
    while scalar:
        if scalar & 1:
            result = _add(result, point)
        point = _add(point, point)
        scalar >>= 1
    return result


def _decode_point(data):
    if len(data) != 32:
        raise ValueError("invalid Ed25519 point length")
    value = int.from_bytes(data, "little")
    sign = value >> 255
    y = value & ((1 << 255) - 1)
    if y >= Q:
        raise ValueError("invalid Ed25519 point")
    x = pow((y * y - 1) * _inv((D * y * y + 1) % Q) % Q, (Q + 3) // 8, Q)
    if (x * x - (y * y - 1) * _inv((D * y * y + 1) % Q)) % Q:
        x = x * I % Q
    if (x * x - (y * y - 1) * _inv((D * y * y + 1) % Q)) % Q:
        raise ValueError("invalid Ed25519 point")
    if (x & 1) != sign:
        x = Q - x
    return (x, y)


def _ed25519_verify(public_key, signature, message):
    if len(public_key) != 32 or len(signature) != 64:
        return False
    try:
        public_point = _decode_point(public_key)
        r_point = _decode_point(signature[:32])
        scalar = int.from_bytes(signature[32:], "little")
        if scalar >= L:
            return False
        import hashlib
        challenge = int.from_bytes(hashlib.sha512(signature[:32] + public_key + message).digest(), "little") % L
        return _mul((B[0], B[1]), scalar) == _add(r_point, _mul(public_point, challenge))
    except ValueError:
        return False


def _decode_minisign_text(value):
    import base64
    text = value.strip()
    if not text.startswith("untrusted comment:"):
        try:
            text = base64.b64decode(text, validate=True).decode("utf-8").strip()
        except Exception as error:
            raise ValueError("invalid minisign text") from error
    return text.splitlines()


def _packet(line, minimum):
    import base64
    try:
        packet = base64.b64decode(line, validate=True)
    except Exception as error:
        raise ValueError("invalid minisign packet") from error
    if len(packet) < minimum:
        raise ValueError("short minisign packet")
    return packet


def _public_key(value):
    lines = _decode_minisign_text(value)
    payload = [line for line in lines if line and "comment:" not in line]
    packet = _packet(payload[0], 42)
    if packet[:2] != b"Ed":
        raise ValueError("updater public key is not Ed25519")
    return packet[2:10], packet[10:]


def _verify_minisign(data, signature_text, public_key_text):
    import base64
    lines = _decode_minisign_text(signature_text)
    if len(lines) != 4 or not lines[0].startswith("untrusted comment:") or not lines[2].startswith("trusted comment: "):
        raise ValueError("invalid minisign signature")
    key_id, public_key = _public_key(public_key_text)
    packet = _packet(lines[1], 74)
    global_signature = _packet(lines[3], 64)
    if packet[:2] not in (b"Ed", b"ED") or packet[2:10] != key_id:
        raise ValueError("signature key ID does not match configured updater public key")
    payload = hashlib.blake2b(data, digest_size=64).digest() if packet[:2] == b"ED" else data
    if not _ed25519_verify(public_key, packet[10:74], payload):
        raise ValueError("updater cryptographic signature is invalid")
    trusted = lines[2][17:].encode("utf-8")
    if not _ed25519_verify(public_key, global_signature, packet[10:74] + trusted):
        raise ValueError("updater trusted comment signature is invalid")
    return True


def _load_public_key(root, explicit=None):
    configured = os.environ.get("MOEPLAY_UPDATER_PUBLIC_KEY")
    if configured:
        return configured.strip()
    path = Path(explicit or os.environ.get("MOEPLAY_UPDATER_PUBLIC_KEY_PATH", root / "updater-public-key.txt"))
    if not path.is_file():
        raise SystemExit(f"Missing fixed updater public key: {path}")
    return path.read_text(encoding="utf-8").strip()

if len(sys.argv) < 3:
    raise SystemExit(__doc__ or "Usage: activate.py ROOT VERSION")
root = Path(sys.argv[1]).resolve()
version = sys.argv[2]
public_key_path = None
if len(sys.argv) > 3:
    if len(sys.argv) != 5 or sys.argv[3] != "--public-key":
        raise SystemExit("Usage: activate.py ROOT VERSION [--public-key path]")
    public_key_path = sys.argv[4]
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
manifest = json.loads((assets / "release-manifest.json").read_text(encoding="utf-8"))
if manifest.get("schemaVersion") != 1 or manifest.get("version") != version or not re.fullmatch(r"[0-9a-f]{40}", manifest.get("commit", "")):
    raise SystemExit("Version mismatch")
if not isinstance(manifest.get("assets"), list) or not manifest["assets"]:
    raise SystemExit("Missing release assets")
latest = json.loads((assets / "latest.json").read_text(encoding="utf-8"))
seen = set()
for asset in manifest["assets"]:
    name = asset.get("file")
    if not isinstance(name, str) or Path(name).name != name or "/" in name or "\\" in name:
        raise SystemExit("Unsafe asset filename")
    if name in seen:
        raise SystemExit("Duplicate asset filename")
    seen.add(name)
    if not re.fullmatch(r"[0-9a-f]{64}", str(asset.get("sha256", "")), re.I) or not isinstance(asset.get("size"), int) or isinstance(asset.get("size"), bool):
        raise SystemExit(f"Invalid asset metadata: {name}")
    file = assets / name
    if not file.is_file():
        raise SystemExit(f"Missing asset: {name}")
    with file.open("rb") as stream:
        digest = hashlib.file_digest(stream, "sha256").hexdigest()
    if file.stat().st_size != asset["size"] or digest != asset["sha256"]:
        raise SystemExit(f"Hash mismatch: {name}")
required_channels = {"installer", "msi", "portable", "release", "compat"}
channels = {asset.get("channel") for asset in manifest["assets"]}
if not required_channels.issubset(channels):
    raise SystemExit("Missing release channel")
site = root / "sites" / version
download = root / "downloads" / version
if site.exists() or download.exists():
    raise SystemExit("Version already exists; old files will not be replaced")
if latest.get("version") != version:
    raise SystemExit("Invalid updater metadata version")
windows_update = latest.get("platforms", {}).get("windows-x86_64")
if not isinstance(windows_update, dict) or not windows_update.get("signature") or not windows_update.get("url"):
    raise SystemExit("Invalid updater metadata")
fixed_public_key = _load_public_key(root, public_key_path)
installer = next((asset for asset in manifest["assets"] if asset.get("channel") == "installer"), None)
try:
    update_url = windows_update["url"]
    parsed_url = urlparse(update_url)
    if parsed_url.scheme != "https" or parsed_url.username or parsed_url.password or Path(parsed_url.path).name != installer["file"]:
        raise ValueError("updater URL does not match installer asset")
    installer_bytes = (assets / installer["file"]).read_bytes()
    _verify_minisign(installer_bytes, windows_update["signature"], fixed_public_key)
except (KeyError, ValueError, OSError) as error:
    raise SystemExit(f"Invalid updater signature: {error}") from error
site.parent.mkdir(parents=True, exist_ok=True)
download.parent.mkdir(parents=True, exist_ok=True)
shutil.copytree(stage / "site", site)
for name in ("release-manifest.json", "latest.json"):
    shutil.copy2(assets / name, site / name)
versions = sorted([p.name for p in site.parent.iterdir() if p.is_dir() and re.fullmatch(r"\d+\.\d+\.\d+", p.name)], key=lambda s: tuple(map(int, s.split("."))))
# Keep every archive page's index in sync.  Historical pages are immutable for
# their release content, but their archive navigation should expose releases
# activated after that page was created as well.
for archive_site in site.parent.iterdir():
    if archive_site.is_dir() and re.fullmatch(r"\d+\.\d+\.\d+", archive_site.name):
        (archive_site / "versions.json").write_text(json.dumps(versions), encoding="utf-8")
shutil.move(str(assets), download)
# Windows SFTP uploads may arrive as owner-only directories. Nginx runs as a
# separate user and needs read/traverse access to this public, validated batch.
for public_root in (site, download):
    public_root.chmod(0o755)
    for public_file in public_root.rglob("*"):
        public_file.chmod(0o755 if public_file.is_dir() else 0o644)
previous = os.readlink(current) if current.is_symlink() else None
next_link = root / f"current-{version}"
next_link.symlink_to(Path("sites") / version, target_is_directory=True)
os.replace(next_link, current)
with (root / "deployment-history.jsonl").open("a") as log:
    log.write(json.dumps({"version": version, "commit": manifest["commit"], "previous": previous, "current": str(site)}) + "\n")
print(f"Activated {version}; previous={previous}")
